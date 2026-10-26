use std::sync::Arc;

use crate::*;

pub type ResolvedIndirectModelRendererBox<'a> =
  Box<dyn ResolvedIndirectModelRenderer + 'a, &'a dyn Allocator>;

/// Resolve the indirect renderer of a scene model for one kind of model (std model, wide line,
/// text, ...).
///
/// A list of resolvers is also a resolver, it selects the first one responsible for the model.
pub trait IndirectModelRendererResolver {
  /// Return None if this resolver is not responsible for the model. The responsibility is decided
  /// by the kind of the model only, so the resolvers are mutually exclusive. Once selected,
  /// missing or invalid data is reported as error instead of falling back to other resolvers.
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>>;
}

impl IndirectModelRendererResolver for [Box<dyn IndirectModelRendererResolver>] {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>> {
    self
      .iter()
      .find_map(|resolver| resolver.resolve(idx, alloc))
  }
}

impl IndirectModelRendererResolver for Vec<Box<dyn IndirectModelRendererResolver>> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>> {
    self.as_slice().resolve(idx, alloc)
  }
}

impl<T: IndirectModelRendererResolver + ?Sized> IndirectModelRendererResolver for Box<T> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>> {
    (**self).resolve(idx, alloc)
  }
}

impl<T: IndirectModelRendererResolver + ?Sized> IndirectModelRendererResolver for Arc<T> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>> {
    (**self).resolve(idx, alloc)
  }
}

/// The indirect renderer resolved for one scene model.
pub trait ResolvedIndirectModelRenderer {
  /// The type id of the resolver, it is hashed into the shader group key to make sure different
  /// impls never produce the same group key.
  fn resolver_type_id(&self) -> TypeId;

  /// see [IndirectBatchSceneModelRenderer::hash_shader_group_key]
  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher);
  fn hash_shader_group_key_with_self_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_shader_group_key(hasher);
    hasher.hash(self.resolver_type_id());
  }

  /// The sub lists whose impl select models have the same impl distinguish key are merged and
  /// share the draw providers created by
  /// [ResolvedIndirectModelRenderer::use_create_or_update_indirect_draw_providers].
  ///
  /// The key must contain the type identity of the impl (for example by hashing
  /// [ResolvedIndirectModelRenderer::resolver_type_id]), because the hook states of the draw
  /// providers are isolated only by the scope keyed by this key.
  fn impl_distinguish_key(&self) -> u64;

  /// This is a hook, it must be called in the scope keyed by the impl distinguish key. The sub
  /// lists's impl select model's impl distinguish key must be all same as this model's.
  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, SceneModelRenderDataError>;

  fn make_draw_command_builder(&self) -> Result<DrawCommandBuilder, SceneModelRenderDataError>;

  /// This component impl often provide id inject logic, if it is not necessary, the
  /// implementation is still required to return `Box::new_in((), alloc)` as the return value.
  fn model_info_injector<'a>(&'a self, alloc: &'a dyn Allocator) -> RenderComponentBox<'a>;

  fn shape_renderable_indirect<'a>(
    &'a self,
    cx: &'a GPUTextureBindingSystem,
    alloc: &'a dyn Allocator,
  ) -> RenderComponentBox<'a>;

  /// see [ResolvedIndirectModelShapeRenderer::get_index_storage_buffer]
  fn get_index_storage_buffer(&self) -> Option<IndicesBufferInfo>;

  fn material_renderable_indirect<'a>(
    &'a self,
    cx: &'a GPUTextureBindingSystem,
    alloc: &'a dyn Allocator,
  ) -> RenderComponentBox<'a>;
}

pub fn use_std_model_renderer(
  cx: &mut QueryGPUHookCx,
  materials: Option<Box<dyn IndirectModelMaterialRendererResolver>>,
  material_key: UseResult<impl DataChanges<Key = u32, Value = u32> + 'static>,
  shapes: Option<Box<dyn IndirectModelShapeRendererResolver>>,
  revere_z: bool,
) -> Option<SceneStdModelIndirectRenderer> {
  let sm_to_std_model_device = use_db_device_foreign_key::<SceneModelStdModelRenderPayload>(cx);

  let (cx, std_model) = cx.use_storage_buffer("std model metadata", 128, u32::MAX);

  cx.use_changes::<StandardModelRefAttributesMeshEntity>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, mesh));

  cx.use_changes::<StandardModelRefSkin>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, skin));

  let state_override = use_state_overrides(cx, revere_z);

  material_key.update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, material));

  std_model.use_update(cx);
  std_model.use_max_item_count_by_db_entity::<StandardModelEntity>(cx);

  cx.when_render(|| SceneStdModelIndirectRenderer {
    model: read_global_db_foreign_key(),
    materials: materials.unwrap(),
    shapes: shapes.unwrap(),
    std_model: std_model.get_gpu_buffer(),
    states: state_override.unwrap(),
    sm_to_std_model_device: sm_to_std_model_device.unwrap(),
  })
}

both!(IndirectStdModelId, u32);

pub struct SceneStdModelIndirectRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
  sm_to_std_model_device: AbstractReadonlyStorageBuffer<[u32]>,
  materials: Box<dyn IndirectModelMaterialRendererResolver>,
  shapes: Box<dyn IndirectModelShapeRendererResolver>,
  states: StateOverrides,
}

impl SceneStdModelIndirectRenderer {
  fn resolve_std_model<'a>(
    &'a self,
    model: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError> {
    let err = SceneModelRenderDataError;
    let shape = self
      .shapes
      .resolve(model, alloc)
      .ok_or(err("no shape impl"))??;
    let material = self
      .materials
      .resolve(model, alloc)
      .ok_or(err("no material impl"))??;
    let states = self.states.get_gpu(model).ok_or(err("state override"))?;

    let resolved = ResolvedSceneStdModelIndirectRenderer {
      renderer: self,
      shape,
      material,
      states,
    };
    Ok(Box::new_in(resolved, alloc))
  }
}

impl IndirectModelRendererResolver for SceneStdModelIndirectRenderer {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelRendererBox<'a>, SceneModelRenderDataError>> {
    let model = self.model.get(idx)?;
    Some(self.resolve_std_model(model, alloc))
  }
}

struct ResolvedSceneStdModelIndirectRenderer<'a> {
  renderer: &'a SceneStdModelIndirectRenderer,
  shape: ResolvedIndirectModelShapeRendererBox<'a>,
  material: ResolvedIndirectModelMaterialRendererBox<'a>,
  states: StateGPUImpl<'a>,
}

impl ResolvedIndirectModelRenderer for ResolvedSceneStdModelIndirectRenderer<'_> {
  fn resolver_type_id(&self) -> TypeId {
    TypeId::of::<SceneStdModelIndirectRenderer>()
  }

  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher) {
    self
      .material
      .hash_shader_group_key_with_self_type_info(hasher);
    self.shape.hash_shader_group_key_with_self_type_info(hasher);
    self.states.hash_pipeline(hasher);
  }

  fn impl_distinguish_key(&self) -> u64 {
    self.shape.impl_distinguish_key()
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, SceneModelRenderDataError> {
    self
      .shape
      .use_create_or_update_indirect_draw_providers(cx, list)
  }

  fn make_draw_command_builder(&self) -> Result<DrawCommandBuilder, SceneModelRenderDataError> {
    self.shape.make_draw_command_builder()
  }

  fn model_info_injector<'a>(&'a self, alloc: &'a dyn Allocator) -> RenderComponentBox<'a> {
    struct SceneStdModelIdInjector<'a> {
      std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
      sm_to_std_model_device: AbstractReadonlyStorageBuffer<[u32]>,
      states: &'a StateGPUImpl<'a>,
    }

    impl<'a> ShaderHashProvider for SceneStdModelIdInjector<'a> {
      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.states.hash_pipeline(hasher);
      }
      shader_hash_type_id! {SceneStdModelIdInjector<'static>}
    }

    impl<'a> ShaderPassBuilder for SceneStdModelIdInjector<'a> {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.std_model);
        ctx.binding.bind(&self.sm_to_std_model_device);
      }
    }

    impl<'a> GraphicsShaderProvider for SceneStdModelIdInjector<'a> {
      fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.vertex(|builder, binding| {
          let buffer = binding.bind_by(&self.std_model);
          let sm_to_std_model_device = binding.bind_by(&self.sm_to_std_model_device);

          let current_id = builder.query::<LogicalRenderEntityId>();
          let std_id = sm_to_std_model_device.index(current_id).load();
          builder.register::<IndirectStdModelId>(std_id);
          let info = buffer.index(std_id).load().expand();
          builder.register::<IndirectAbstractMaterialId>(info.material);
          builder.register::<IndirectSkinId>(info.skin);
          builder.set_vertex_out::<IndirectAbstractMaterialId>(info.material);
          builder.register::<IndirectAbstractMeshId>(info.mesh);
        });
        self.states.build(builder);
      }
    }

    Box::new_in(
      SceneStdModelIdInjector {
        std_model: self.renderer.std_model.clone(),
        states: &self.states,
        sm_to_std_model_device: self.renderer.sm_to_std_model_device.clone(),
      },
      alloc,
    )
  }

  fn shape_renderable_indirect<'a>(
    &'a self,
    _cx: &'a GPUTextureBindingSystem,
    alloc: &'a dyn Allocator,
  ) -> RenderComponentBox<'a> {
    self.shape.make_component_indirect(alloc)
  }

  fn get_index_storage_buffer(&self) -> Option<IndicesBufferInfo> {
    self.shape.get_index_storage_buffer()
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    cx: &'a GPUTextureBindingSystem,
    alloc: &'a dyn Allocator,
  ) -> RenderComponentBox<'a> {
    self.material.make_component_indirect(cx, alloc)
  }
}

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct SceneStdModelStorage {
  pub mesh: u32, // todo, improve: this data is duplicate with the mesh dispatcher sm-ref-mesh data
  pub material: u32,
  pub skin: u32,
}

only_vertex!(IndirectSkinId, u32);
