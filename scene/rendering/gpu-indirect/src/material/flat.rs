use crate::*;

pub type FlatMaterialStorageBuffer = ReactiveStorageBufferContainer<FlatMaterialStorage>;

pub fn flat_material_storage_buffer(cx: &GPU) -> FlatMaterialStorageBuffer {
  let color = global_watch()
    .watch::<FlatMaterialDisplayColorComponent>()
    .collective_map(srgb4_to_linear4);
  let color_offset = offset_of!(FlatMaterialStorage, color);

  let alpha = global_watch().watch::<AlphaOf<FlatMaterialAlphaConfig>>();
  let alpha_offset = offset_of!(FlatMaterialStorage, alpha);

  let alpha_cutoff = global_watch().watch::<AlphaCutoffOf<FlatMaterialAlphaConfig>>();
  let alpha_cutoff_offset = offset_of!(FlatMaterialStorage, alpha_cutoff);

  ReactiveStorageBufferContainer::new(cx)
    .with_source(color, color_offset)
    .with_source(alpha, alpha_offset)
    .with_source(alpha_cutoff, alpha_cutoff_offset)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct FlatMaterialStorage {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

#[derive(Clone)]
pub struct FlatMaterialStorageGPU {
  pub buffer: StorageBufferReadOnlyDataView<[FlatMaterialStorage]>,
  pub alpha_mode: AlphaMode,
}

impl ShaderHashProvider for FlatMaterialStorageGPU {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl GraphicsShaderProvider for FlatMaterialStorageGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let materials = binding.bind_by(&self.buffer);
      let current_material_id = builder.query::<IndirectAbstractMaterialId>();
      let material = materials.index(current_material_id).load().expand();

      builder.register::<DefaultDisplay>(material.color);

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: material.alpha_cutoff,
        alpha: material.alpha,
      }
      .apply(builder);
    })
  }
}

impl ShaderPassBuilder for FlatMaterialStorageGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.buffer);
  }
}
