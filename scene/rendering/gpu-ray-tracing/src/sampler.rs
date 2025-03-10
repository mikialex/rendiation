use rendiation_lighting_transport::DeviceSampler;
use rendiation_shader_library::sampling::hammersley_2d_fn;

use crate::*;

/// Fast, reasonably good hash that updates 32 bits of state and outputs 32 bits.
///
/// This is a version of `pcg32i_random_t` from the
/// [PCG random number generator library](https://www.pcg-random.org/index.html),
/// which updates its internal state using a linear congruential generator and
/// outputs a hash using `pcg_output_rxs_m_xs_32_32`, a more complex hash.
fn pcg(state: &ShaderPtrOf<u32>) -> Node<u32> {
  let prev = state.load() * val(747796405_u32) + val(2891336453_u32);
  let word = ((prev >> ((prev >> val(28_u32)) + val(4_u32))) ^ prev) * val(277803737_u32);
  state.store(prev);
  (word >> val(22_u32)) ^ word
}

///  High-quality hash that takes 96 bits of data and outputs 32, roughly twice
///  as slow as `pcg`.
///
/// You can use this to generate a seed for subsequent random number generators;
/// for instance, provide `uvec3(pixel.x, pixel.y, frame_number).
///
/// From https://github.com/Cyan4973/xxHash and https://www.shadertoy.com/view/XlGcRh.
pub fn xxhash32(p: Node<Vec3<u32>>) -> Node<u32> {
  let primes = val(Vec4::new(
    2246822519_u32,
    3266489917_u32,
    668265263_u32,
    374761393_u32,
  ));

  let h32 = p.z() + primes.w() + p.x() * primes.y();
  let h32 = primes.z() * ((h32 << val(17)) | (h32 >> val(32 - 17)));
  let h32 = h32 + p.y() * primes.y();
  let h32 = primes.z() * ((h32 << val(17)) | (h32 >> val(32 - 17)));
  let h32 = primes.x() * (h32 ^ (h32 >> val(15)));
  let h32 = primes.y() * (h32 ^ (h32 >> val(13)));
  h32 ^ (h32 >> val(16))
}

/// https://github.com/nvpro-samples/vk_raytrace/blob/master/shaders/random.glsl
pub struct PCGRandomSampler {
  state: ShaderPtrOf<u32>,
  seed: Node<u32>,
}
impl PCGRandomSampler {
  pub fn new(seed: Node<u32>) -> Self {
    Self {
      state: seed.make_local_var(),
      seed,
    }
  }
}
impl DeviceSampler for PCGRandomSampler {
  fn reset(&self, _: Node<u32>) {
    self.state.store(self.seed);
  }
  fn next(&self) -> Node<f32> {
    let r = pcg(&self.state);
    (val(0x3f800000_u32) | (r >> val(9))).bitcast::<f32>() - val(1.0)
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
