use wgpu_types::TextureFormat;

use crate::*;
struct Test;

struct TestSemantic;
impl SemanticVertexShaderValue for TestSemantic {
  type ValueType = Vec4<f32>;
}

impl SemanticFragmentShaderValue for TestSemantic {
  type ValueType = Vec4<f32>;
}

impl GraphicsShaderProvider for Test {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.set_vertex_out::<TestSemantic>(Vec4::default());
      // let varying = builder.set_vertex_out_anonymous(Vec4::default());
      // Ok(varying)
      Ok(())
    })?;
    builder.fragment(|builder, _| {
      let v = builder.get_fragment_in::<TestSemantic>()?;
      // let v2 = builder.get_fragment_in_anonymous(varying);
      builder.define_out_by(channel(TextureFormat::Rgba32Float));
      builder.set_fragment_out(0, v)
    })
  }
}

#[test]
fn test() {
  test_provider_success(&Test);
}
