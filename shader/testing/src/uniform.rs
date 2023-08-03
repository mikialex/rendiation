use wgpu_types::TextureFormat;

use crate as shadergraph;
use crate::*;

#[derive(ShaderStruct, Clone, Copy, Default)]
pub struct TestUniform {
  pub data: f32,
  pub data2: Vec2<f32>,
  pub data3: Vec3<f32>,
}

impl ShaderBindingProvider for TestUniform {
  type Node = Self;
}

pub struct FakeTexture2d;

impl ShaderBindingProvider for FakeTexture2d {
  type Node = ShaderTexture2D;
}

pub struct FakeSampler;

impl ShaderBindingProvider for FakeSampler {
  type Node = ShaderSampler;
}

impl GraphicsShaderProvider for TestUniform {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.binding::<Self>();

    builder.vertex(|builder, binding| {
      let tex = binding.binding::<FakeTexture2d>();
      let sampler = binding.binding::<FakeSampler>();

      let uniform = uniform.using().expand();
      let color = tex.sample(sampler, uniform.data2);
      builder.register::<ClipPosition>(color);
      builder.register::<ClipPosition>((uniform.data3, uniform.data));
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let uniform = uniform.using().expand();
      builder.define_out_by(channel(TextureFormat::Rgba32Float));
      builder.set_fragment_out(0, (uniform.data3, 1.))?;
      Ok(())
    })
  }
}

#[test]
fn test() {
  test_provider_success(&TestUniform::default());
}
