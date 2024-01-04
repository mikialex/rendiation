use crate::*;

mod inv_cdf;
pub use inv_cdf::*;
mod sir;
pub use sir::*;
mod mis;
pub use mis::*;
mod ris;
pub use ris::*;

/// not required to be normalized
pub trait Distribution {
  type Sample: SampleType;
  fn eval(&self, at: Self::Sample) -> f32;
}

pub trait NormalizedDistribution: Distribution {
  // todo validation fn
}

/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Monte_Carlo_Basics#TheMonteCarloEstimator
pub trait MonteCarloEstimator {
  type Sample: SampleType;

  /// single estimation the integral of (distribution, region)
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = Self::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32;

  /// using the single estimation to compute integral by given sample_count
  fn integrate(
    &self,
    region: &impl SampleRegionType<Value = Self::Sample>,
    sample_count: usize,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let mut sum = 0.;
    for next_sampling_index in 0..sample_count {
      sampler.reset(next_sampling_index);
      sum += self.estimate(region, sampler);
    }
    sum / (sample_count as f32)
  }
}

/// The most naive estimator
pub struct BasicEstimator<'a, D>(&'a D);
impl<'a, D> MonteCarloEstimator for BasicEstimator<'a, D>
where
  D: Distribution,
{
  type Sample = D::Sample;
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    self.0.eval(region.sample_uniformly(sampler))
  }
}

pub struct ImportanceEstimator<'a, D>(&'a D);

pub trait ImportanceSampledDistribution: Distribution {
  /// we not pass the sampler in parameter because we may share one uniform sample among multiple
  /// importance sampled distribution
  fn importance_sample(&self, uniform_sample: Self::Sample) -> Self::Sample;
  fn pdf(&self, sampled_at: Self::Sample) -> f32;
}

impl<'a, D> MonteCarloEstimator for ImportanceEstimator<'a, D>
where
  D: ImportanceSampledDistribution,
{
  type Sample = D::Sample;
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let distribution = self.0;
    let importance_sample = distribution.importance_sample(region.sample_uniformly(sampler));
    let pdf = distribution.pdf(importance_sample);
    let value = distribution.eval(importance_sample);
    value / pdf
  }
}

pub struct RussianRouletteEstimator<E> {
  pub threshold: f32,
  /// 0 is often used
  pub constant: f32,
  pub estimator: E,
}

impl<E> MonteCarloEstimator for RussianRouletteEstimator<E>
where
  E: MonteCarloEstimator,
{
  type Sample = E::Sample;
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = E::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    if rand::random::<f32>() > self.threshold {
      (self.estimator.estimate(region, sampler) - self.threshold * self.constant)
        / (1.0 - self.threshold)
    } else {
      self.constant
    }
  }
}
