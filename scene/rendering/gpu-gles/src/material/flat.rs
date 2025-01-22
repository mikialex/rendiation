use crate::*;

pub type FlatMaterialUniforms =
  UniformUpdateContainer<EntityHandle<FlatMaterialEntity>, FlatMaterialUniform>;

pub fn flat_material_uniforms(cx: &GPU) -> FlatMaterialUniforms {
  let color = global_watch()
    .watch::<FlatMaterialDisplayColorComponent>()
    .collective_map(srgb4_to_linear4)
    .into_query_update_uniform(offset_of!(FlatMaterialUniform, color), cx);

  let alpha = global_watch()
    .watch::<AlphaOf<FlatMaterialAlphaConfig>>()
    .into_query_update_uniform(offset_of!(FlatMaterialUniform, alpha), cx);

  let alpha_cutoff = global_watch()
    .watch::<AlphaCutoffOf<FlatMaterialAlphaConfig>>()
    .into_query_update_uniform(offset_of!(FlatMaterialUniform, alpha_cutoff), cx);

  FlatMaterialUniforms::default()
    .with_source(color)
    .with_source(alpha)
    .with_source(alpha_cutoff)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

pub fn flat_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<FlatMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<FlatMaterialAlphaConfig>>()
}

#[derive(Clone)]
pub struct FlatMaterialGPU<'a> {
  pub uniform: &'a UniformBufferDataView<FlatMaterialUniform>,
  pub alpha_mode: AlphaMode,
}

impl ShaderHashProvider for FlatMaterialGPU<'_> {
  shader_hash_type_id! {FlatMaterialGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl GraphicsShaderProvider for FlatMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: uniform.alpha_cutoff,
        alpha: uniform.alpha,
      }
      .apply(builder);
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}
