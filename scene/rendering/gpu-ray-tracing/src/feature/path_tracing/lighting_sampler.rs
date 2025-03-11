use crate::*;

pub trait AbstractLightSamplingStrategy {
  fn sample_light_index_impl(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<u32>;
  fn pmf(&self, world_position: Node<Vec3<f32>>, light_idx: Node<u32>) -> Node<f32>;
  fn sample_light_index(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (Node<f32>, Node<u32>) {
    let r = self.sample_light_index_impl(world_position, sampler);
    (self.pmf(world_position, r), r)
  }
}

pub struct LightingGroup<T: ShaderSizedValueNodeType> {
  strategy: Arc<dyn AbstractLightSamplingStrategy>,
  lights: ShaderReadonlyPtrOf<[T]>,
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
  T: ShaderSizedValueNodeType,
  Node<T>: DevicePathTracingLightingInvocation,
{
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let (pmf, light_idx) = self.strategy.sample_light_index(world_position, sampler);
    let light = self.lights.index(light_idx).load();

    let result = light.importance_sampling_light(world_position, sampler);

    RTLightSampling {
      sampling_dir: result.sampling_dir,
      pdf: result.pdf * pmf,
      radiance: result.radiance,
    }
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
  /// if return u32::MAX, then no light is picked
  fn sample_light_index_impl(
    &self,
    _world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<u32> {
    let light_idx = (sampler.next() * self.light_count.into_f32())
      .min(self.light_count.into_f32() - val(1.))
      .into_u32();
    self.light_count.equals(0).select(val(u32::MAX), light_idx)
  }

  fn pmf(&self, _world_position: Node<Vec3<f32>>, _light_idx: Node<u32>) -> Node<f32> {
    self
      .light_count
      .equals(0)
      .select(val(0.), val(1.) / self.light_count.into_f32())
  }
}
