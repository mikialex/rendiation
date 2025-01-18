mod precomputed;
mod sobol;
pub use precomputed::*;
use rand::Rng;
use rendiation_algebra::{Lerp, Vec2};
pub use sobol::*;

/// https://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface#fragment-SamplerInterface-2
///
/// Because sample values must be strictly less than 1,
/// OneMinusEpsilon, that represents the largest representable floating-point constant that is less
/// than 1. Later, we will clamp sample vector values to be no larger than this value.
/// const ONE_MINUS_EPSILON: f32 = 0x1.ffffffep - 1;
///
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

pub trait SampleType: Copy {}

#[derive(Default)]
pub struct SampleWithResult<T> {
  pub sample_at: T,
  pub sample_result: f32,
}

impl SampleType for f32 {}
impl SampleType for usize {}
impl<T: SampleType> SampleType for Vec2<T> {}

pub trait SampleRegionType: Copy {
  type Value: SampleType;

  fn sample_uniformly(&self, sampler: &mut impl Sampler) -> Self::Value;
}

#[derive(Clone, Copy)]
pub struct OneDimensionContinuesRegion<T> {
  pub start: T,
  pub end: T,
}

impl SampleRegionType for OneDimensionContinuesRegion<f32> {
  type Value = f32;

  fn sample_uniformly(&self, sampler: &mut impl Sampler) -> Self::Value {
    self.start.lerp(self.end, sampler.next())
  }
}

#[derive(Clone, Copy)]
pub struct TwoDimensionContinuesRegion<T> {
  pub x: OneDimensionContinuesRegion<T>,
  pub y: OneDimensionContinuesRegion<T>,
}

impl SampleRegionType for TwoDimensionContinuesRegion<f32> {
  type Value = Vec2<f32>;

  fn sample_uniformly(&self, sampler: &mut impl Sampler) -> Self::Value {
    Vec2::new(
      self.x.sample_uniformly(sampler),
      self.y.sample_uniformly(sampler),
    )
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

/// pmf should be normalized, return index
///
/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Sampling_Using_the_Inversion_Method#DiscreteCase
pub fn importance_sample_discrete(pmf: &[f32], sample: f32) -> usize {
  let mut idx = 0;
  let mut sum = 0.;
  // note, sample can not get 1, but it's ok
  // and we also not care about the rounding error
  for weight in pmf {
    sum += weight;
    if sum <= sample {
      idx += 1
    } else {
      break;
    }
  }
  idx
}
