use crate::*;

pub trait AbstractLightSamplingStrategy {
  // return if light sample success/valid, when there is no lighting, return false
  fn sample_light_index_impl(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (Node<u32>, Node<bool>);
  fn pmf(&self, world_position: Node<Vec3<f32>>, light_idx: Node<u32>) -> Node<f32>;
  fn sample_light_index(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (Node<f32>, Node<u32>, Node<bool>) {
    let (r, valid) = self.sample_light_index_impl(world_position, sampler);
    (self.pmf(world_position, r), r, valid)
  }
}

pub struct LightingGroup<T> {
  pub strategy: Arc<dyn AbstractLightSamplingStrategy>,
  pub lights: ShaderReadonlyPtrOf<[T]>,
}

impl<T: ShaderSizedValueNodeType> Clone for LightingGroup<T> {
  fn clone(&self) -> Self {
    Self {
      strategy: self.strategy.clone(),
      lights: self.lights.clone(),
    }
  }
}

impl<T> DevicePathTracingLightingInvocation for LightingGroup<T>
where
  T: ShaderSizedValueNodeType + ShaderStructuralNodeType,
  ENode<T>: DevicePathTracingLightingInvocation,
{
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    let (pmf, light_idx, valid) = self.strategy.sample_light_index(world_position, sampler);
    let light = valid.select_branched(|| self.lights.index(light_idx).load(), zeroed_val);

    let (result, inner_valid) = light
      .expand()
      .importance_sampling_light(world_position, sampler);

    (
      RTLightSampling {
        sampling_dir: result.sampling_dir,
        pdf: result.pdf * pmf,
        radiance: result.radiance,
      },
      valid.and(inner_valid),
    )
  }
}

/// the simplest possible light sampler: it samples all lights with uniform probability.
/// In practice, more sophisticated sampling algorithms are usually much more effective,
/// but this one is easy to implement and provides a useful baseline for comparing
/// light sampling techniques.
pub struct UniformLightSamplingStrategy {
  pub light_count: Node<u32>,
}

impl AbstractLightSamplingStrategy for UniformLightSamplingStrategy {
  fn sample_light_index_impl(
    &self,
    _world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (Node<u32>, Node<bool>) {
    let light_idx = (sampler.next() * self.light_count.into_f32())
      .min(self.light_count.into_f32() - val(1.))
      .into_u32();
    (light_idx, self.light_count.not_equals(0))
  }

  fn pmf(&self, _world_position: Node<Vec3<f32>>, _light_idx: Node<u32>) -> Node<f32> {
    self
      .light_count
      .equals(0)
      .select(val(0.), val(1.) / self.light_count.into_f32())
  }
}
