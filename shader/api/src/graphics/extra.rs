use type_identity::*;

use crate::*;

impl<T: GraphicsShaderProvider> GraphicsShaderProvider for TypeHashProvideByTypeName<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.0.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.0.post_build(builder)
  }
}
impl<T: GraphicsShaderProvider> GraphicsShaderProvider for TypeHashProvideByTypeId<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.0.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.0.post_build(builder)
  }
}
