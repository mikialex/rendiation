use crate::*;

/// SIR
///
/// https://dezeming.top/wp-content/uploads/2021/11/%E9%87%8D%E8%A6%81%E6%80%A7%E9%87%8D%E9%87%87%E6%A0%B7%E6%8A%80%E6%9C%AF.pdf
pub struct SamplingImportanceResampling<D: Distribution, PD> {
  pub target_distribution: D,
  pub proposed_distribution: PD,
  /// the "M"
  ///
  /// if M = 1, this actually becomes importance sampling with proposed_distribution as pdf
  pub proposed_count: usize,
}

pub struct SamplingImportanceResamplingResult<'a, D: Distribution, PD> {
  _source: &'a SamplingImportanceResampling<D, PD>,
  pre_sample_samples: Vec<SampleWithResult<D::Sample>>,
  weights: Vec<f32>,
  weight_average_inv: f32,
}

impl<'a, D: Distribution, PD> SamplingImportanceResamplingResult<'a, D, PD> {
  pub fn get_target_distribution_importance_sample(&self, idx: usize) -> D::Sample {
    self.pre_sample_samples[idx].sample_at
  }
}

impl<D, PD> SamplingImportanceResampling<D, PD>
where
  D: Distribution,
  PD: ImportanceSampledDistribution<Sample = D::Sample> + NormalizedDistribution,
{
  pub fn pre_sample_iter<'a>(
    &'a self,
    sampler: &'a mut impl Sampler,
    region: impl SampleRegionType<Value = D::Sample> + 'a,
  ) -> impl Iterator<Item = (SampleWithResult<D::Sample>, f32)> + 'a {
    (0..self.proposed_count).map(move |_| {
      let uniform = region.sample_uniformly(sampler);
      let sample_at = self.proposed_distribution.importance_sample(uniform);
      let sample_result = self.proposed_distribution.eval(sample_at);
      let weight = self.target_distribution.eval(sample_at) / sample_result;

      let sample = SampleWithResult {
        sample_at,
        sample_result,
      };

      (sample, weight)
    })
  }

  pub fn pre_sample(
    &self,
    sampler: &mut impl Sampler,
    region: impl SampleRegionType<Value = D::Sample>,
  ) -> SamplingImportanceResamplingResult<D, PD> {
    let mut pre_sample_samples = Vec::with_capacity(self.proposed_count);
    let mut weights = Vec::with_capacity(self.proposed_count);
    let mut weight_sum = 0.;
    for (sample, weight) in self.pre_sample_iter(sampler, region) {
      weight_sum += weight;
      pre_sample_samples.push(sample);
      weights.push(weight);
    }

    SamplingImportanceResamplingResult {
      _source: self,
      pre_sample_samples,
      weights,
      weight_average_inv: (self.proposed_count as f32) / weight_sum,
    }
  }
}

impl<'a, D: Distribution, PD> Distribution for SamplingImportanceResamplingResult<'a, D, PD> {
  type Sample = usize;

  /// we assume the input is uniform 0 to 1
  fn eval(&self, at: Self::Sample) -> f32 {
    self.pre_sample_samples[at].sample_result
  }
}

// this should only sample once
impl<'a, D: Distribution, PD> ImportanceSampledDistribution
  for SamplingImportanceResamplingResult<'a, D, PD>
{
  fn importance_sample(&self, uniform_sample: Self::Sample) -> Self::Sample {
    importance_sample_discrete(&self.weights, uniform_sample as f32)
  }

  // note, this importance sampling is sample thr target dist, not the discrete set
  // so the pdf is the average of all discrete
  fn pdf(&self, _sampled_at: Self::Sample) -> f32 {
    self.weight_average_inv
  }
}
