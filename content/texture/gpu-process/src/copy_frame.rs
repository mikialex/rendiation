use crate::*;

pub struct CopyFrame {
  source: RenderTargetView,
}

impl ShaderHashProvider for CopyFrame {
  shader_hash_type_id! {}
}

pub fn copy_frame(source: RenderTargetView, blend: Option<BlendState>) -> impl PassContent {
  CopyFrame { source }.draw_quad_with_blend(blend)
}

impl ShaderPassBuilder for CopyFrame {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
    ctx.binding.bind(&self.source);
  }
}

impl GraphicsShaderProvider for CopyFrame {
  fn build(&self, builder: &mut rendiation_shader_api::ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let source = binding.bind_by(&self.source);

      let uv = builder.query::<FragmentUv>();
      let value = source.sample(sampler, uv);
      builder.store_fragment_out(0, value)
    })
  }
}
