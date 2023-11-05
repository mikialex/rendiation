use crate::*;

mod precomputed;
mod sobol;
pub use precomputed::*;
use rand::Rng;
pub use sobol::*;

/// https://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface#fragment-SamplerInterface-2

/// Because sample values must be strictly less than 1,
/// OneMinusEpsilon, that represents the largest representable floating-point constant that is less
/// than 1. Later, we will clamp sample vector values to be no larger than this value.
// const ONE_MINUS_EPSILON: f32 = 0x1.ffffffep - 1;

/// The task of a Sampler is to generate a sequence of -dimensional samples in
/// [0, 1) ^ d
pub trait Sampler {
  fn reset(&mut self, next_sampling_index: usize);

  fn next(&mut self) -> f32;

  /// While a 2D sample value could be constructed by using values returned by a pair of calls to
  /// sample(), some samplers can generate better point distributions if they know that two
  /// dimensions will be used together.
  fn next_2d(&mut self) -> (f32, f32) {
    (self.next(), self.next())
  }
  fn next_vec2(&mut self) -> Vec2<f32> {
    Vec2::from(self.next_2d())
  }
}

#[derive(Default)]
pub struct RngSampler {
  // todo, we should control the seed for stable output
  rng: rand::rngs::ThreadRng,
}

impl Sampler for RngSampler {
  fn next(&mut self) -> f32 {
    self.rng.gen()
  }

  fn next_2d(&mut self) -> (f32, f32) {
    (self.rng.gen(), self.rng.gen())
  }

  fn reset(&mut self, _next_sampling_index: usize) {}
}
