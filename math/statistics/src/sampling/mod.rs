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

pub trait SampleType: Copy {
  type Scope: SampleRegionType;
}

pub trait SampleRegionType: Copy {
  type Value: SampleType;

  fn sample_uniformly(&self) -> Self::Value;
}

/// not required to be normalized
pub trait Distribution {
  type Sample: SampleType;
  fn eval(&self, at: Self::Sample) -> f32;
}

/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Monte_Carlo_Basics#TheMonteCarloEstimator
pub trait MonteCarloEstimator<D: Distribution> {
  fn estimate(&self, distribution: &D, region: &impl SampleRegionType<Value = D::Sample>) -> f32;

  fn integrate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sample_count: usize,
  ) -> f32 {
    let mut sum = 0.;
    for _ in 0..sample_count {
      sum += self.estimate(distribution, region);
    }
    sum / (sample_count as f32)
  }
}

pub struct BasicEstimator;
impl<D> MonteCarloEstimator<D> for BasicEstimator
where
  D: Distribution,
{
  fn estimate(&self, distribution: &D, region: &impl SampleRegionType<Value = D::Sample>) -> f32 {
    distribution.eval(region.sample_uniformly())
  }
}

pub struct ImportanceEstimator;

pub trait ImportanceSampledDistribution: Distribution {
  fn importance_sample(&self, uniform_sample: Self::Sample) -> Self::Sample;
  fn pdf(&self, sampled_at: Self::Sample) -> f32;
}

impl<D> MonteCarloEstimator<D> for ImportanceEstimator
where
  D: ImportanceSampledDistribution,
{
  fn estimate(&self, distribution: &D, region: &impl SampleRegionType<Value = D::Sample>) -> f32 {
    let importance_sample = distribution.importance_sample(region.sample_uniformly());
    let pdf = distribution.pdf(importance_sample);
    let value = distribution.eval(importance_sample);
    value / pdf
  }
}

pub struct DistributionMultiply<A, B> {
  pub dist_a: A,
  pub dist_b: B,
}

impl<A, B> Distribution for DistributionMultiply<A, B>
where
  A: Distribution,
  B: Distribution<Sample = A::Sample>,
{
  type Sample = A::Sample;

  fn eval(&self, at: Self::Sample) -> f32 {
    self.dist_a.eval(at) * self.dist_b.eval(at)
  }
}

pub struct MultiImportanceEstimator;

/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Improving_Efficiency#MultipleImportanceSampling
///
/// we could use a trait to support more than two sample source
impl<A, B> MonteCarloEstimator<DistributionMultiply<A, B>> for MultiImportanceEstimator
where
  DistributionMultiply<A, B>: Distribution<Sample = A::Sample>,
  A: ImportanceSampledDistribution<Sample = B::Sample>,
  B: ImportanceSampledDistribution,
{
  /// balance heuristic
  fn estimate(
    &self,
    distribution: &DistributionMultiply<A, B>,
    region: &impl SampleRegionType<Value = A::Sample>,
  ) -> f32 {
    let uniform = region.sample_uniformly();
    let sample_a = distribution.dist_a.importance_sample(uniform);
    let sample_b = distribution.dist_b.importance_sample(uniform);

    let sample_a_pdf_a = distribution.dist_a.pdf(sample_a);
    let sample_b_pdf_b = distribution.dist_b.pdf(sample_b);

    let sample_a_pdf_b = distribution.dist_b.pdf(sample_a);
    let sample_b_pdf_a = distribution.dist_a.pdf(sample_b);

    distribution.dist_a.eval(sample_a) / (sample_a_pdf_a + sample_a_pdf_b)
      + distribution.dist_b.eval(sample_b) / (sample_b_pdf_a + sample_b_pdf_b)
  }
}

pub struct RussianRouletteEstimator<E> {
  pub threshold: f32,
  /// 0 is often used
  pub constant: f32,
  pub estimator: E,
}

impl<D, E> MonteCarloEstimator<D> for RussianRouletteEstimator<E>
where
  D: Distribution,
  E: MonteCarloEstimator<D>,
{
  fn estimate(&self, distribution: &D, region: &impl SampleRegionType<Value = D::Sample>) -> f32 {
    if rand::random::<f32>() > self.threshold {
      (self.estimator.estimate(distribution, region) - self.threshold * self.constant)
        / (1.0 - self.threshold)
    } else {
      self.constant
    }
  }
}
