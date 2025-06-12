use crate::*;

pub fn use_unlit_material_storage(
  cx: &mut impl QueryGPUHookCx,
) -> Option<UnlitMaterialIndirectRenderer> {
  let storages = cx.use_storage_buffer(unlit_material_storages);
  let tex_storages = cx.use_storage_buffer(unlit_material_texture_storages);

  cx.when_render(|| UnlitMaterialIndirectRenderer {
    material_access: global_entity_component_of::<StandardModelRefUnlitMaterial>()
      .read_foreign_key(),
    storages: storages.unwrap(),
    texture_handles: tex_storages.unwrap(),
    alpha_mode: global_entity_component_of().read(),
  })
}

#[derive(Clone)]
pub struct UnlitMaterialIndirectRenderer {
  material_access: ForeignKeyReadView<StandardModelRefUnlitMaterial>,
  storages: StorageBufferReadonlyDataView<[UnlitMaterialStorage]>,
  texture_handles: StorageBufferReadonlyDataView<[UnlitMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<UnlitMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for UnlitMaterialIndirectRenderer {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let m = self.material_access.get(any_idx)?;
    Some(Box::new(UnlitMaterialStorageGPU {
      buffer: self.storages.clone(),
      alpha_mode: self.alpha_mode.get_value(m)?,
      texture_handles: self.texture_handles.clone(),
      binding_sys: cx,
    }))
  }
  fn hash_shader_group_key(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
    _: &mut PipelineHasher,
  ) -> Option<()> {
    self.material_access.get(any_idx)?;
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

type UnlitMaterialStorageBuffer = ReactiveStorageBufferContainer<UnlitMaterialStorage>;

fn unlit_material_storages(cx: &GPU) -> UnlitMaterialStorageBuffer {
  let color = global_watch()
    .watch::<UnlitMaterialColorComponent>()
    .collective_map(srgb4_to_linear4)
    .into_query_update_storage(offset_of!(UnlitMaterialStorage, color));

  let alpha = global_watch()
    .watch::<AlphaOf<UnlitMaterialAlphaConfig>>()
    .into_query_update_storage(offset_of!(UnlitMaterialStorage, alpha));

  let alpha_cutoff = global_watch()
    .watch::<AlphaCutoffOf<UnlitMaterialAlphaConfig>>()
    .into_query_update_storage(offset_of!(UnlitMaterialStorage, alpha_cutoff));

  create_reactive_storage_buffer_container(128, u32::MAX, cx)
    .with_source(color)
    .with_source(alpha)
    .with_source(alpha_cutoff)
}

type TexStorage = UnlitMaterialTextureHandlesStorage;
type UnlitMaterialTexStorages = ReactiveStorageBufferContainer<TexStorage>;

fn unlit_material_texture_storages(cx: &GPU) -> UnlitMaterialTexStorages {
  let c = create_reactive_storage_buffer_container(128, u32::MAX, cx);
  let base_color_alpha = offset_of!(TexStorage, color_alpha_texture);
  add_tex_watcher::<UnlitMaterialColorAlphaTex, _>(c, base_color_alpha)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
struct UnlitMaterialStorage {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
struct UnlitMaterialTextureHandlesStorage {
  pub color_alpha_texture: TextureSamplerHandlePair,
}

#[derive(Clone)]
struct UnlitMaterialStorageGPU<'a> {
  pub buffer: StorageBufferReadonlyDataView<[UnlitMaterialStorage]>,
  pub texture_handles: StorageBufferReadonlyDataView<[UnlitMaterialTextureHandlesStorage]>,
  pub alpha_mode: AlphaMode,
  pub binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for UnlitMaterialStorageGPU<'_> {
  shader_hash_type_id! {UnlitMaterialStorageGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
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

      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();
      let color_alpha_tex = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.color_alpha_texture,
        uv,
        val(Vec4::one()),
      );

      builder.register::<DefaultDisplay>(material.color * color_alpha_tex);

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
