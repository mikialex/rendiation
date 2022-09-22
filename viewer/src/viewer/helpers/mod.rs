use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;
use shadergraph::*;
use webgpu::*;

pub mod axis;
pub mod camera;
pub mod grid;

pub type HelperLineMesh = FatlineMesh;
pub type HelperLineModel = FatlineImpl;

/// just add premultiplied alpha to shader
pub struct WidgetDispatcher {
  inner: DefaultPassDispatcher,
}
impl DispatcherDynSelf for WidgetDispatcher {}
impl WidgetDispatcher {
  pub fn new(inner: DefaultPassDispatcher) -> Self {
    Self { inner }
  }
}

impl ShaderHashProvider for WidgetDispatcher {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for WidgetDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.setup_pass(ctx);
  }
}

impl ShaderGraphProvider for WidgetDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.build(builder)
  }
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.post_build(builder)?;
    builder.fragment(|builder, _| {
      // todo improve, we should only override blend
      MaterialStates {
        blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        ..Default::default()
      }
      .apply_pipeline_builder(builder);

      let old = builder.get_fragment_out(0)?;
      let new = (old.xyz() * old.w(), old.w());
      builder.set_fragment_out(0, new)
    })
  }
}
