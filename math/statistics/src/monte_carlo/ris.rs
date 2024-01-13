use crate::*;

/// D is the target distribution
///
/// G is good approximation of D, but can not be importance sampled
///
/// P is not good(but not too bad) approximation of D (so it's also the approximation of the G), but
/// could be importance sampled and is normalized
///
/// we use sir to get importance sampling of G, and use importance sampled G to estimate D
pub struct ResampledImportanceSampling<D, G, P>
where
  D: Distribution,
  G: Distribution,
  P: ImportanceSampledDistribution + NormalizedDistribution,
{
  pub target_dis: D,
  pub sir: SamplingImportanceResampling<G, P>,
}

impl<D, G, P> MonteCarloEstimator for ResampledImportanceSampling<D, G, P>
where
  D: Distribution,
  G: Distribution<Sample = D::Sample>,
  P: ImportanceSampledDistribution + NormalizedDistribution<Sample = D::Sample>,
{
  type Sample = D::Sample;
  fn estimate(
    &self,
    region: &impl SampleRegionType<Value = D::Sample>,
    sampler: &mut impl Sampler,
  ) -> f32 {
    let pre_sampled = self.sir.pre_sample(sampler, *region);

    let uniform_sample = sampler.next();
    let uniform_sample = (uniform_sample / self.sir.proposed_count as f32).floor() as usize;

    let idx = pre_sampled.importance_sample(uniform_sample);
    let sample = pre_sampled.eval(idx) / pre_sampled.pdf(idx);

    let target_sample_at = pre_sampled.get_target_distribution_importance_sample(idx);
    let target_eval = self.target_dis.eval(target_sample_at);

    target_eval / sample
  }
}
