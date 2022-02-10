use crate as shadergraph;
use crate::*;

#[derive(ShaderUniform, Clone, Copy, Default)]
pub struct TestUniform {
  pub data: f32,
  pub data2: Vec3<f32>,
}

impl SemanticShaderUniform for TestUniform {
  const TYPE: SemanticBinding = SemanticBinding::Object;
}

impl ShaderGraphProvider for TestUniform {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.register_uniform::<Self>().expand();

    builder.vertex_position.set((uniform.data2, uniform.data));
    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let value = (builder.query_uniform::<Self>()?.expand().data2, 1.).into();
    builder.set_fragment_out(0, value);
    Ok(())
  }
}

#[test]
fn test() {
  let result = build_shader(&TestUniform::default(), &WGSL).unwrap();

  println!("vertex: \n{}", result.vertex_shader);
  println!("fragment: \n{}", result.frag_shader);
}
