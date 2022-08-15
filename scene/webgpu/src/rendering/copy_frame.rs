use crate::*;

pub struct CopyFrame<T> {
  sampler: ImmediateSampler,
  source: AttachmentView<T>,
}
struct CopyFrameTypeMark;
impl<T> ShaderHashProvider for CopyFrame<T> {
  fn hash_pipeline(&self, _: &mut PipelineHasher) {}
}

impl<T> ShaderHashProviderAny for CopyFrame<T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    CopyFrameTypeMark.type_id().hash(hasher);
  }
}

pub fn copy_frame<T>(source: AttachmentView<T>, blend: Option<BlendState>) -> impl PassContent {
  CopyFrame {
    source,
    sampler: Default::default(),
  }
  .draw_quad_with_blend(blend)
}

#[derive(Default, Clone)]
pub struct ImmediateSampler {
  inner: TextureSampler,
}

impl ShaderUniformProvider for ImmediateSampler {
  type Node = ShaderSampler;
}

impl From<ImmediateSampler> for SamplerDescriptor<'static> {
  fn from(val: ImmediateSampler) -> Self {
    val.inner.into()
  }
}

impl<T> ShaderPassBuilder for CopyFrame<T> {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&self.sampler, SB::Material);
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
