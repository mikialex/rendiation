// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail/#more-3285

use crate::*;

pub struct TAA {
  history: Attachment,
}

impl TAA {
  pub fn resolve(&mut self, new: &mut Attachment, ctx: &mut FrameCtx) -> &Attachment {
    let mut resolve_target = attachment()
      .format(webgpu::TextureFormat::Rgba8Unorm)
      .request(ctx);

    pass("taa-resolve")
      .with_color(resolve_target.write(), load())
      .render(ctx)
      .by(
        TAAResolver {
          history: self.history.read(),
          new: new.read(),
        }
        .draw_quad(),
      );

    std::mem::swap(&mut self.history, &mut resolve_target);

    &self.history
  }
}

struct TAAResolver<'a> {
  history: AttachmentView<&'a Attachment>,
  new: AttachmentView<&'a Attachment>,
}

impl<'a> ShaderGraphProvider for TAAResolver<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let sampler = binding.uniform_by(&self.sampler, SB::Material);
      let history = binding.uniform_by(&self.history, SB::Material);
      let new = binding.uniform_by(&self.new, SB::Material);

      let uv = builder.query::<FragmentUv>()?;
      let new = new.sample(sampler, uv);

      builder.set_fragment_out(0, new)
    })
  }
}

impl<'a> ShaderPassBuilder for TAAResolver<'a> {
  fn setup_pass(&self, _ctx: &mut GPURenderPassCtx) {}
}
impl<'a> ShaderHashProvider for TAAResolver<'a> {}
impl<'a> ShaderHashProviderAny for TAAResolver<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    todo!()
  }
}
