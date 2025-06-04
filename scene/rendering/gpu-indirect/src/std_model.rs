use crate::*;

pub trait IndirectModelRenderImpl {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;

  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder>;

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl IndirectModelRenderImpl for Vec<Box<dyn IndirectModelRenderImpl>> {
  fn device_id_injector(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.device_id_injector(any_id) {
        return Some(v);
      }
    }
    None
  }
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    for provider in self {
      if let Some(v) = provider.hash_shader_group_key_with_self_type_info(any_id, hasher) {
        return Some(v);
      }
    }
    None
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable_indirect(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(v) = provider.material_renderable_indirect(any_idx, cx) {
        return Some(v);
      }
    }
    None
  }
}

#[derive(Default)]
pub struct DefaultSceneStdModelIndirectRendererProvider {
  pub std_model: QueryToken,
  pub materials: Vec<BoxedQueryBasedGPUFeature<Box<dyn IndirectModelMaterialRenderImpl>>>,
  pub shapes: Vec<BoxedQueryBasedGPUFeature<Box<dyn IndirectModelShapeRenderImpl>>>,
}

impl DefaultSceneStdModelIndirectRendererProvider {
  pub fn register_material_impl(
    mut self,
    imp: impl QueryBasedFeature<Box<dyn IndirectModelMaterialRenderImpl>, Context = GPU> + 'static,
  ) -> Self {
    self.materials.push(Box::new(imp));
    self
  }
  pub fn register_shape_impl(
    mut self,
    imp: impl QueryBasedFeature<Box<dyn IndirectModelShapeRenderImpl>, Context = GPU> + 'static,
  ) -> Self {
    self.shapes.push(Box::new(imp));
    self
  }
}

impl QueryBasedFeature<Box<dyn IndirectModelRenderImpl>>
  for DefaultSceneStdModelIndirectRendererProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.std_model = qcx.register_multi_updater(std_model_data(cx));
    self.materials.iter_mut().for_each(|p| p.register(qcx, cx));
    self.shapes.iter_mut().for_each(|p| p.register(qcx, cx));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.std_model);
    self.materials.iter_mut().for_each(|p| p.deregister(qcx));
    self.shapes.iter_mut().for_each(|p| p.deregister(qcx));
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: self.materials.iter().map(|v| v.create_impl(cx)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(cx)).collect(),
      std_model: cx.take_storage_array_buffer(self.std_model).unwrap(),
    })
  }
}
struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  std_model: StorageBufferReadonlyDataView<[SceneStdModelStorage]>,
  materials: Vec<Box<dyn IndirectModelMaterialRenderImpl>>,
  shapes: Vec<Box<dyn IndirectModelShapeRenderImpl>>,
}

impl IndirectModelRenderImpl for SceneStdModelRenderer {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let model = self.model.get(any_id)?;
    self
      .materials
      .hash_shader_group_key_with_self_type_info(model, hasher)?;
    self
      .shapes
      .hash_shader_group_key_with_self_type_info(model, hasher)?;
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn device_id_injector(
    &self,
    _: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    struct SceneStdModelIdInjector {
      std_model: StorageBufferReadonlyDataView<[SceneStdModelStorage]>,
    }

    impl ShaderHashProvider for SceneStdModelIdInjector {
      shader_hash_type_id! {}
    }

    impl ShaderPassBuilder for SceneStdModelIdInjector {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.std_model);
      }
    }

    impl GraphicsShaderProvider for SceneStdModelIdInjector {
      fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.vertex(|builder, binding| {
          let buffer = binding.bind_by(&self.std_model);
          let sm_id = builder.query::<IndirectSceneStdModelId>();
          let info = buffer.index(sm_id).load().expand();
          builder.register::<IndirectAbstractMaterialId>(info.material);
          builder.set_vertex_out::<IndirectAbstractMaterialId>(info.material);
          builder.register::<IndirectAbstractMeshId>(info.mesh);
        });
      }
    }

    Some(Box::new(SceneStdModelIdInjector {
      std_model: self.std_model.clone(),
    }))
  }

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_component_indirect(model)
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_draw_command_builder(model)
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(any_idx)?;
    self.materials.make_component_indirect(model, cx)
  }
}

only_vertex!(IndirectSceneStdModelId, u32);
pub type SceneStdModelStorageBuffer = ReactiveStorageBufferContainer<SceneStdModelStorage>;

pub fn std_model_data(cx: &GPU) -> SceneStdModelStorageBuffer {
  let mesh = global_watch()
    .watch::<StandardModelRefAttributesMeshEntity>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .into_query_update_storage(offset_of!(SceneStdModelStorage, mesh));

  let material_flat = global_watch()
    .watch::<StandardModelRefUnlitMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material_pbr_mr = global_watch()
    .watch::<StandardModelRefPbrMRMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material_pbr_sg = global_watch()
    .watch::<StandardModelRefPbrSGMaterial>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_boxed();

  let material = material_flat
    .collective_select(material_pbr_mr)
    .collective_select(material_pbr_sg)
    .into_query_update_storage(offset_of!(SceneStdModelStorage, material));

  create_reactive_storage_buffer_container(128, u32::MAX, cx)
    .with_source(mesh)
    .with_source(material)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct SceneStdModelStorage {
  pub mesh: u32, // todo, improve: this data is duplicate with the mesh dispatcher sm-ref-mesh data
  pub material: u32,
}

pub struct StdModelGPUStorage<'a> {
  pub buffer: &'a SceneStdModelStorageBuffer,
}

impl ShaderHashProvider for StdModelGPUStorage<'_> {
  shader_hash_type_id! {StdModelGPUStorage<'static>}
}

impl GraphicsShaderProvider for StdModelGPUStorage<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<IndirectSceneStdModelId>();
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectAbstractMeshId>(model.mesh);
      builder.register::<IndirectAbstractMaterialId>(model.material);
    })
  }
}

impl ShaderPassBuilder for StdModelGPUStorage<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
