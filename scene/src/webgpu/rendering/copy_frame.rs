use rendiation_texture::TextureSampler;
use rendiation_webgpu::*;
use shadergraph::{FragmentUv, ShaderGraphProvider, SB};

use crate::{AttachmentReadView, PassContent};

pub struct CopyFrame<T> {
  sampler: TextureSampler,
  source: AttachmentReadView<T>,
}

impl<T> PassContent for CopyFrame<T> {
  fn render(&mut self, pass: &mut crate::SceneRenderPass) {
    todo!()
  }
}

pub fn copy_frame<T>(source: AttachmentReadView<T>) -> CopyFrame<T> {
  CopyFrame {
    source,
    sampler: Default::default(),
  }
}

pub struct QuadDraw;

impl ShaderPassBuilder for QuadDraw {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.pass.draw(0..4, 0..1);
  }
}

impl<T> ShaderPassBuilder for CopyFrame<T> {
  fn setup_pass(&self, ctx: &mut rendiation_webgpu::GPURenderPassCtx) {
    let sampler = GPUSampler::create(self.sampler.into(), &ctx.gpu.device);
    let sampler = sampler.create_view(Default::default());

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
      // let sampler = binding.uniform_by(&self.sampler, SB::Material).expand();
      // let source = binding.uniform_by(&self.source, SB::Material).expand();

      // let uv = builder.query::<FragmentUv>()?;
      // let value = source.sample(sampler, uv);
      // builder.set_fragment_out(0, value)
      Ok(())
    })
  }
}
