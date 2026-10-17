use crate::*;

pub fn use_unlit_material_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<UnlitMaterialIndirectRenderer> {
  let (cx, storages) = cx.use_storage_buffer("unlit materials parameter data", 128, u32::MAX);

  cx.use_changes::<UnlitMaterialColorComponent>()
    .map_changes(srgb4_to_linear4)
    .update_storage_array(cx, storages, offset_of!(UnlitMaterialStorage, color));

  cx.use_changes::<AlphaOf<UnlitMaterialAlphaConfig>>()
    .update_storage_array(cx, storages, offset_of!(UnlitMaterialStorage, alpha));

  cx.use_changes::<AlphaCutoffOf<UnlitMaterialAlphaConfig>>()
    .update_storage_array(cx, storages, offset_of!(UnlitMaterialStorage, alpha_cutoff));

  storages.use_max_item_count_by_db_entity::<UnlitMaterialEntity>(cx);
  storages.use_update(cx);

  let (cx, tex_storages) = cx.use_storage_buffer("unlit materials texture data", 128, u32::MAX);

  let base_color_alpha = offset_of!(TexStorage, color_alpha_texture);
  use_tex_watcher::<UnlitMaterialColorAlphaTex, _>(cx, tex_storages, base_color_alpha);

  tex_storages.use_max_item_count_by_db_entity::<UnlitMaterialEntity>(cx);
  tex_storages.use_update(cx);

  cx.when_render(|| UnlitMaterialIndirectRenderer {
    material_access: read_global_db_foreign_key(),
    storages: storages.get_gpu_buffer(),
    texture_handles: tex_storages.get_gpu_buffer(),
    alpha_mode: read_global_db_component(),
  })
}

#[derive(Clone)]
pub struct UnlitMaterialIndirectRenderer {
  material_access: ForeignKeyReadView<StandardModelRefUnlitMaterial>,
  storages: AbstractReadonlyStorageBuffer<[UnlitMaterialStorage]>,
  texture_handles: AbstractReadonlyStorageBuffer<[UnlitMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<UnlitMaterialAlphaConfig>>,
}

impl UnlitMaterialIndirectRenderer {
  fn read_material(
    &self,
    idx: EntityHandle<UnlitMaterialEntity>,
  ) -> Result<ResolvedUnlitMaterialIndirectRenderer<'_>, SceneModelRenderDataError> {
    let err = SceneModelRenderDataError;
    Ok(ResolvedUnlitMaterialIndirectRenderer {
      renderer: self,
      alpha_mode: self
        .alpha_mode
        .get_value(idx)
        .ok_or(err("unlit material alpha mode"))?,
    })
  }
}

impl IndirectModelMaterialRendererResolver for UnlitMaterialIndirectRenderer {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelMaterialRendererBox<'a>, SceneModelRenderDataError>> {
    let idx = self.material_access.get(idx)?;
    Some(self.read_material(idx).map(|r| Box::new_in(r, alloc) as _))
  }
}

struct ResolvedUnlitMaterialIndirectRenderer<'a> {
  renderer: &'a UnlitMaterialIndirectRenderer,
  alpha_mode: AlphaMode,
}

impl ResolvedIndirectModelMaterialRenderer for ResolvedUnlitMaterialIndirectRenderer<'_> {
  fn resolver_type_id(&self) -> TypeId {
    TypeId::of::<UnlitMaterialIndirectRenderer>()
  }

  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.alpha_mode);
  }

  fn make_component_indirect<'a>(
    &'a self,
    cx: &'a GPUTextureBindingSystem,
  ) -> Box<dyn RenderComponent + 'a> {
    Box::new(UnlitMaterialStorageGPU {
      buffer: self.renderer.storages.clone(),
      alpha_mode: self.alpha_mode,
      texture_handles: self.renderer.texture_handles.clone(),
      binding_sys: cx,
    })
  }
}

type TexStorage = UnlitMaterialTextureHandlesStorage;

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default)]
struct UnlitMaterialStorage {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct UnlitMaterialTextureHandlesStorage {
  pub color_alpha_texture: TextureSamplerHandlePair,
}

#[derive(Clone)]
struct UnlitMaterialStorageGPU<'a> {
  pub buffer: AbstractReadonlyStorageBuffer<[UnlitMaterialStorage]>,
  pub texture_handles: AbstractReadonlyStorageBuffer<[UnlitMaterialTextureHandlesStorage]>,
  pub alpha_mode: AlphaMode,
  pub binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for UnlitMaterialStorageGPU<'_> {
  shader_hash_type_id! {UnlitMaterialStorageGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.alpha_mode);
  }
}

impl GraphicsShaderProvider for UnlitMaterialStorageGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    // allow this material to be used with none uv geometry provider
    builder.vertex(|builder, _| {
      if builder.try_query::<GeometryUV>().is_none() {
        builder.register::<GeometryUV>(val(Vec2::zero()));
      }
    });
    builder.fragment(|builder, binding| {
      let materials = binding.bind_by(&self.buffer);
      let tex_handles = binding.bind_by(&self.texture_handles);
      let current_material_id = builder.query::<IndirectAbstractMaterialId>();
      let material = materials.index(current_material_id).load().expand();
      let tex_storage = tex_handles.index(current_material_id).load().expand();

      let uv = builder.get_or_compute_fragment_uv();
      let color_alpha_tex = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.color_alpha_texture,
        uv,
        val(Vec4::one()),
      );

      builder.register::<DefaultDisplay>(material.color * color_alpha_tex);
      builder.insert_type_tag::<UnlitMaterialTag>();

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: material.alpha_cutoff,
        alpha: material.alpha * color_alpha_tex.w(),
      }
      .apply(builder);
    })
  }
}

impl ShaderPassBuilder for UnlitMaterialStorageGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.buffer);
    ctx.binding.bind(&self.texture_handles);
  }
}
