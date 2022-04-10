use rendiation_texture::TextureSampler;
use rendiation_webgpu::*;
use shadergraph::{FragmentUv, ShaderGraphProvider, ShaderSampler, ShaderUniformProvider, SB};

use crate::{AttachmentReadView, PassContent, UseQuadDraw};

pub struct CopyFrame<T> {
  sampler: ImmediateSampler,
  source: AttachmentReadView<T>,
}

pub fn copy_frame<T>(source: AttachmentReadView<T>) -> impl PassContent {
  CopyFrame {
    source,
    sampler: Default::default(),
  }
  .draw_quad()
}

#[derive(Default)]
pub struct ImmediateSampler {
  inner: TextureSampler,
}

impl ShaderUniformProvider for ImmediateSampler {
  type Node = ShaderSampler;
}

impl<T> ShaderPassBuilder for CopyFrame<T> {
  fn setup_pass(&self, ctx: &mut rendiation_webgpu::GPURenderPassCtx) {
    let sampler = GPUSampler::create(self.sampler.inner.into(), &ctx.gpu.device);
    let sampler = sampler.create_default_view();

    ctx.binding.bind(&sampler, SB::Material);
    ctx.binding.bind(&self.source, SB::Material);
  }
}

impl<T> ShaderGraphProvider for CopyFrame<T> {
  fn build(
    &self,
    builder: &mut shadergraph::ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let sampler = binding.uniform_by(&self.sampler, SB::Material);
      let source = binding.uniform_by(&self.source, SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      let value = source.sample(sampler, uv);
      builder.set_fragment_out(0, value)
    })
  }
}
