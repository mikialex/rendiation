use rendiation_lighting_transport::DeviceSampler;
use rendiation_shader_library::sampling::hammersley_2d_fn;

use crate::*;

pub struct UniformRangeSampler {
  state: ShaderPtrOf<u32>,
  seed: Node<u32>,
}
impl UniformRangeSampler {
  pub fn new(seed: Node<u32>) -> Self {
    Self {
      state: seed.make_local_var(),
      seed,
    }
  }
}
impl DeviceSampler for UniformRangeSampler {
  fn reset(&self, _: Node<u32>) {
    self.state.store(self.seed);
  }
  /// https://github.com/JMS55/bevy/blob/solari3/crates/bevy_pbr/src/solari/global_illumination/utils.wgsl#L8-L36
  fn next(&self) -> Node<f32> {
    self
      .state
      .store(self.state.load() * val(747796405_u32) + val(2891336453_u32));
    let state = self.state.load();
    let word = ((state >> ((state >> val(28_u32)) + val(4_u32))) ^ state) * val(277803737_u32);
    let r = ((word >> val(22_u32)) ^ word).bitcast::<f32>() * val(0x2f800004_u32).bitcast::<f32>();
    // shader_assert(r.less_equal_than(val(1.0)));
    r.fract().abs()
  }
}

pub struct TestSampler {
  pub sample_count: Node<u32>,
}

impl DeviceSampler for TestSampler {
  fn reset(&self, _: Node<u32>) {}

  fn next(&self) -> Node<f32> {
    hammersley_2d_fn(self.sample_count, val(64)).x()
  }

  fn next_2d(&self) -> Node<Vec2<f32>> {
    hammersley_2d_fn(self.sample_count, val(64))
  }
}
