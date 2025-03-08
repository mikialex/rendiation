use crate::*;

pub type UnlitMaterialStorageBuffer = ReactiveStorageBufferContainer<UnlitMaterialStorage>;

pub fn unlit_material_storage_buffer(cx: &GPU) -> UnlitMaterialStorageBuffer {
  let color = global_watch()
    .watch::<UnlitMaterialColorComponent>()
    .collective_map(srgb4_to_linear4);
  let color_offset = offset_of!(UnlitMaterialStorage, color);

  let alpha = global_watch().watch::<AlphaOf<UnlitMaterialAlphaConfig>>();
  let alpha_offset = offset_of!(UnlitMaterialStorage, alpha);

  let alpha_cutoff = global_watch().watch::<AlphaCutoffOf<UnlitMaterialAlphaConfig>>();
  let alpha_cutoff_offset = offset_of!(UnlitMaterialStorage, alpha_cutoff);

  ReactiveStorageBufferContainer::new(cx)
    .with_source(color, color_offset)
    .with_source(alpha, alpha_offset)
    .with_source(alpha_cutoff, alpha_cutoff_offset)
}

type TexStorage = UnlitMaterialTextureHandlesStorage;
pub type UnlitMaterialTexStorages = ReactiveStorageBufferContainer<TexStorage>;
pub fn unlit_material_texture_storages(cx: &GPU) -> UnlitMaterialTexStorages {
  let c = UnlitMaterialTexStorages::new(cx);
  let base_color_alpha = offset_of!(TexStorage, color_alpha_texture);
  add_tex_watcher::<UnlitMaterialColorAlphaTex, _>(c, base_color_alpha)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct UnlitMaterialStorage {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct UnlitMaterialTextureHandlesStorage {
  pub color_alpha_texture: TextureSamplerHandlePair,
}

#[derive(Clone)]
pub struct UnlitMaterialStorageGPU<'a> {
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
