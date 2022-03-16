use rendiation_texture::TextureSampler;
use rendiation_webgpu::GPUSampler;
use shadergraph::{FragmentUv, ShaderGraphProvider, SB};

use crate::{AttachmentReadView, ShaderPassBuilder};

pub struct CopyFrame<T> {
  sampler: TextureSampler,
  source: AttachmentReadView<T>,
}

pub fn copy_frame<T>(source: AttachmentReadView<T>) -> CopyFrame<T> {
  CopyFrame {
    source,
    sampler: Default::default(),
  }
}

impl<T> ShaderPassBuilder for CopyFrame<T> {
  fn setup_pass(&self, ctx: &mut rendiation_webgpu::GPURenderPassCtx) {
    let sampler = GPUSampler::create(self.sampler.into(), &ctx.gpu.device);
    let sampler = sampler.create_view(Default::default());

    // ctx.binding.setup_uniform(&sampler, SB::Material);
    // ctx.binding.setup_uniform(&self.source, SB::Material);
    ctx.binding.setup_pass(ctx.pass, &ctx.gpu.device, todo!());
    ctx.pass.draw(0..4, 0..1);
  }
}

impl<T> ShaderGraphProvider for CopyFrame<T> {
  fn build(
    &self,
    builder: &mut shadergraph::ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      // let sampler = binding.uniform_by(&self.sampler, SB::Material).expand();
      // let source = binding.uniform_by(&self.source, SB::Material).expand();

      // let uv = builder.query::<FragmentUv>()?;
      // let value = source.sample(sampler, uv);
      // builder.set_fragment_out(0, value)
      Ok(())
    })
  }
}
