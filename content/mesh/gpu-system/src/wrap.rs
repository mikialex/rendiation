use core::{any::Any, hash::Hash};

use crate::*;

pub struct BindlessMeshProvider<'a, T> {
  pub base: &'a T,
  pub system: &'a BindlessMeshDispatcher,
}

impl<'a, T: ShaderPassBuilder> ShaderPassBuilder for BindlessMeshProvider<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
    self.system.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.post_setup_pass(ctx)
  }
}

impl<'a, T: ShaderHashProviderAny> ShaderHashProviderAny for BindlessMeshProvider<'a, T> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher);
    self.base.hash_pipeline_with_type_info(hasher)
  }
}

impl<'a, T: ShaderHashProvider> ShaderHashProvider for BindlessMeshProvider<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.base.hash_pipeline(hasher);
    self.system.hash_pipeline(hasher)
  }
}
impl<'a, T: GraphicsShaderProvider> GraphicsShaderProvider for BindlessMeshProvider<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.base.build(builder)?;
    self.system.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.base.post_build(builder)
  }
}
