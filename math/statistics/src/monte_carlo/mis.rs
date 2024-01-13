use crate::*;

/// MIS using balance heuristic
pub struct MultipleImportanceSamplingTwo<A, B> {
  pub dist_a: A,
  pub dist_b: B,
}

impl<A, B> Distribution for MultipleImportanceSamplingTwo<A, B>
where
  A: Distribution,
  B: Distribution<Sample = A::Sample>,
{
  type Sample = A::Sample;

  fn eval(&self, at: Self::Sample) -> f32 {
    self.dist_a.eval(at) * self.dist_b.eval(at)
  }
}

/// https://pbr-book.org/4ed/Monte_Carlo_Integration/Improving_Efficiency#MultipleImportanceSampling
///
/// we could use a trait to support more than two sample source
impl<A, B> MonteCarloEstimator for MultipleImportanceSamplingTwo<A, B>
where
  A: ImportanceSampledDistribution<Sample = B::Sample>,
  B: ImportanceSampledDistribution,
{
  type Sample = A::Sample;

  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = A::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let uniform = region.sample_uniformly(sampler);
    let sample_a = self.dist_a.importance_sample(uniform);
    let sample_b = self.dist_b.importance_sample(uniform);

    let sample_a_pdf_a = self.dist_a.pdf(sample_a);
    let sample_b_pdf_b = self.dist_b.pdf(sample_b);

    let sample_a_pdf_b = self.dist_b.pdf(sample_a);
    let sample_b_pdf_a = self.dist_a.pdf(sample_b);

    self.dist_a.eval(sample_a) / (sample_a_pdf_a + sample_a_pdf_b)
      + self.dist_b.eval(sample_b) / (sample_b_pdf_a + sample_b_pdf_b)
  }
}
