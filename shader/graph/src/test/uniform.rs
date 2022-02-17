use super::test_provider_success;
use crate as shadergraph;
use crate::*;

#[derive(ShaderUniform, Clone, Copy, Default)]
pub struct TestUniform {
  pub data: f32,
  pub data2: Vec2<f32>,
  pub data3: Vec3<f32>,
}

impl ShaderUniformProvider for TestUniform {
  type Node = Self;
}

pub struct FakeTexture2d;

impl ShaderUniformProvider for FakeTexture2d {
  type Node = ShaderTexture;
}

pub struct FakeSampler;

impl ShaderUniformProvider for FakeSampler {
  type Node = ShaderSampler;
}

impl ShaderGraphProvider for TestUniform {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.register_uniform::<Self>(SB::Object).expand();
    let tex = builder.register_uniform::<FakeTexture2d>(SB::Object);
    let sampler = builder.register_uniform::<FakeSampler>(SB::Object);

    let color = tex.sample(sampler, uniform.data2);
    builder.vertex_position.set(color);
    builder.vertex_position.set((uniform.data3, uniform.data));
    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let value = (
      builder.query_uniform::<Self>(SB::Object)?.expand().data3,
      1.,
    )
      .into();
    builder.set_fragment_out(0, value)?;
    Ok(())
  }
}

#[test]
fn test() {
  test_provider_success(&TestUniform::default());
}
