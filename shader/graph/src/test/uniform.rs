use super::test_provider_success;
use crate as shadergraph;
use crate::*;

#[derive(ShaderStruct, Clone, Copy, Default)]
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
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.uniform::<Self>(SB::Object);

    builder.vertex(|builder, binding| {
      let tex = binding.uniform::<FakeTexture2d>(SB::Object);
      let sampler = binding.uniform::<FakeSampler>(SB::Object);

      let uniform = uniform.using().expand();
      let color = tex.sample(sampler, uniform.data2);
      builder.register::<ClipPosition>(color);
      builder.register::<ClipPosition>((uniform.data3, uniform.data));
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let uniform = uniform.using().expand();
      builder.set_fragment_out(0, (uniform.data3, 1.))?;
      Ok(())
    })
  }
}

#[test]
fn test() {
  test_provider_success(&TestUniform::default());
}
