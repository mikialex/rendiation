use crate::*;

pub type UnlitMaterialUniforms =
  UniformUpdateContainer<EntityHandle<UnlitMaterialEntity>, UnlitMaterialUniform>;

type TexUniform = UnlitMaterialTextureHandlesUniform;
pub type UnlitMaterialTexUniforms =
  UniformUpdateContainer<EntityHandle<UnlitMaterialEntity>, TexUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct UnlitMaterialUniform {
  pub color: Vec4<f32>,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}
#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct UnlitMaterialTextureHandlesUniform {
  pub color_alpha_texture: TextureSamplerHandlePair,
}

pub fn unlit_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<UnlitMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<UnlitMaterialAlphaConfig>>()
}

#[derive(Clone)]
pub struct UnlitMaterialGPU<'a> {
  pub uniform: &'a UniformBufferDataView<UnlitMaterialUniform>,
  pub tex_uniform: &'a UniformBufferDataView<UnlitMaterialTextureHandlesUniform>,
  pub alpha_mode: AlphaMode,
  pub color_alpha_tex_sampler: (u32, u32),
  pub binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for UnlitMaterialGPU<'_> {
  shader_hash_type_id! {UnlitMaterialGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
}

impl GraphicsShaderProvider for UnlitMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    // allow this material to be used with none uv geometry provider
    builder.vertex(|builder, _| {
      if builder.try_query::<GeometryUV>().is_none() {
        builder.register::<GeometryUV>(val(Vec2::zero()));
      }
    });
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();
      let tex_uniform = binding.bind_by(&self.tex_uniform).load().expand();
      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let color_alpha_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.color_alpha_tex_sampler,
        tex_uniform.color_alpha_texture,
        uv,
        val(Vec4::one()),
      );

      builder.register::<DefaultDisplay>(uniform.color * color_alpha_tex);

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: uniform.alpha_cutoff,
        alpha: uniform.alpha * color_alpha_tex.w(),
      }
      .apply(builder);
    })
  }
}

impl ShaderPassBuilder for UnlitMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(self.tex_uniform);
    setup_tex(ctx, self.binding_sys, self.color_alpha_tex_sampler);
  }
}
