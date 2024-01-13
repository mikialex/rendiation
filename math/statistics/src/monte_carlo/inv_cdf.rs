use crate::*;

pub struct ImportanceSamplingByInverseCDF<P: NormalizedDistribution, F> {
  pub pdf: P,
  pub inverse_cdf: F,
}

impl<P: NormalizedDistribution, F> ImportanceSamplingByInverseCDF<P, F> {
  /// test inverse_cdf impl correctness
  pub fn numerical_validation(&self) {
    todo!()
  }
}

impl<P, F> Distribution for ImportanceSamplingByInverseCDF<P, F>
where
  P: NormalizedDistribution,
{
  type Sample = P::Sample;

  fn eval(&self, at: Self::Sample) -> f32 {
    self.pdf.eval(at)
  }
}

impl<P, F> ImportanceSampledDistribution for ImportanceSamplingByInverseCDF<P, F>
where
  P: NormalizedDistribution,
  F: Fn(Self::Sample) -> Self::Sample,
{
  fn importance_sample(&self, uniform_sample: Self::Sample) -> Self::Sample {
    (self.inverse_cdf)(uniform_sample)
  }

  fn pdf(&self, sampled_at: Self::Sample) -> f32 {
    self.eval(sampled_at)
  }
}
