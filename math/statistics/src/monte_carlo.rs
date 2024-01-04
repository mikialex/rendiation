use crate::*;

/// not required to be normalized
pub trait Distribution {
  type Sample: SampleType;
  fn eval(&self, at: Self::Sample) -> f32;
}

/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Monte_Carlo_Basics#TheMonteCarloEstimator
pub trait MonteCarloEstimator<D: Distribution> {
  /// single estimation the integral of (distribution, region)
  fn estimate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32;

  /// using the single estimation to compute integral by given sample_count
  fn integrate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sample_count: usize,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let mut sum = 0.;
    for next_sampling_index in 0..sample_count {
      sampler.reset(next_sampling_index);
      sum += self.estimate(distribution, region, sampler);
    }
    sum / (sample_count as f32)
  }
}

/// The most naive estimator
pub struct BasicEstimator;
impl<D> MonteCarloEstimator<D> for BasicEstimator
where
  D: Distribution,
{
  fn estimate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    distribution.eval(region.sample_uniformly(sampler))
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
  fn estimate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let importance_sample = distribution.importance_sample(region.sample_uniformly(sampler));
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
    sampler: &mut impl Sampler,
  ) -> f32 {
    let uniform = region.sample_uniformly(sampler);
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
  fn estimate(
    &self,
    distribution: &D,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    if rand::random::<f32>() > self.threshold {
      (self.estimator.estimate(distribution, region, sampler) - self.threshold * self.constant)
        / (1.0 - self.threshold)
    } else {
      self.constant
    }
  }
}
