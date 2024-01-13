use crate::*;

/// same as ResampledImportanceSampling but using Reservoir to do pre sampling
pub struct WeightedReservoirSampling<D, G, P>
where
  D: Distribution,
  G: Distribution,
  P: ImportanceSampledDistribution + NormalizedDistribution,
{
  pub target_dis: D,
  pub sir: SamplingImportanceResampling<G, P>,
}

impl<D, G, P> MonteCarloEstimator for WeightedReservoirSampling<D, G, P>
where
  D: Distribution,
  G: Distribution<Sample = D::Sample>,
  P: ImportanceSampledDistribution + NormalizedDistribution<Sample = D::Sample>,
  D::Sample: Default, // should use Zero trait?
{
  type Sample = D::Sample;
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let mut res = Reservoir::<SampleWithResult<D::Sample>>::default();

    self
      .sir
      .pre_sample_iter(sampler, *region)
      .for_each(|(sample, weight)| res.update(sample, weight));

    let target_eval = self.target_dis.eval(res.sample.sample_at);
    target_eval / res.sample.sample_result
  }
}

#[derive(Default)]
pub struct Reservoir<T> {
  sample: T,
  weight_sum: f32,
}

impl<T> Reservoir<T> {
  pub fn update(&mut self, sample: T, weight: f32) {
    self.weight_sum += weight;
    if rand::random::<f32>() < weight / self.weight_sum {
      self.sample = sample
    }
  }
}
