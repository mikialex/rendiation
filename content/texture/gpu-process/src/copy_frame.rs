use crate::*;

pub struct CopyFrame {
  pub source: RenderTargetView,
  pub viewport: Option<Vec4<f32>>,
}

impl ShaderHashProvider for CopyFrame {
  shader_hash_type_id! {}
}

pub fn copy_frame(source: RenderTargetView, blend: Option<BlendState>) -> impl PassContent {
  CopyFrame {
    source,
    viewport: None,
  }
  .draw_quad_with_blend(blend)
}

impl ShaderPassBuilder for CopyFrame {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
    if let Some(viewport) = self.viewport {
      let [x, y, w, h] = viewport.into();
      ctx.pass.set_viewport(x, y, w, h, 0., 1.);
    }

    ctx.binding.bind(&self.source);
    let (w, h) = ctx.pass.size().into_f32();
    if self.viewport.is_some() {
      ctx.pass.set_viewport(0., 0., w, h, 0., 1.);
    }
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
