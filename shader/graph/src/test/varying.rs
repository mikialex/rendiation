use super::test_provider_success;
use crate::*;

struct Test;

struct TestSemantic;
impl SemanticVertexShaderValue for TestSemantic {
  type ValueType = Vec4<f32>;
}

impl SemanticFragmentShaderValue for TestSemantic {
  type ValueType = Vec4<f32>;
}

impl ShaderGraphProvider for Test {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.set_vertex_out::<TestSemantic>(Vec4::default());
    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let v = builder.get_fragment_in::<TestSemantic>()?;
    builder.set_fragment_out(0, v)?;
    Ok(())
  }
}

#[test]
fn test() {
  test_provider_success(&Test);
}
