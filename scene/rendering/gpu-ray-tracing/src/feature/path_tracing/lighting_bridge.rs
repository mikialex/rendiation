use rendiation_lighting_punctual::PointLightShaderInfo;

use crate::*;

pub trait DevicePathTracingLighting: ShaderHashProvider + DynClone {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingLightingInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(DevicePathTracingLighting);

pub trait DevicePathTracingLightingInvocation: DynClone {
  // also return if light sample success/valid, when there is no lighting, return false
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>);
}
dyn_clone::clone_trait_object!(DevicePathTracingLightingInvocation);

pub struct RTLightSampling {
  pub sampling_dir: Node<Vec3<f32>>,
  pub pdf: Node<f32>,
  pub radiance: Node<Vec3<f32>>,
}

pub trait ShaderLightSource {
  /// in watt
  fn radiant_power(&self) -> Node<Vec3<f32>>;

  /// for dirac distribution, return 0
  fn pdf(&self, sample_dir: Node<Vec3<f32>>) -> Node<f32>;

  fn importance_sampling_light_impl(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>>;

  fn compute_sample_radiance(
    &self,
    surface_position: Node<Vec3<f32>>,
    sample_dir: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>>;

  fn importance_sampling_light(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let sampling_dir = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);
    let pdf = self.pdf(sampling_dir);
    RTLightSampling {
      sampling_dir,
      pdf,
      radiance,
    }
  }
}

// impl DevicePathTracingLightingInvocation for ENode<PointLightShaderInfo> {
//   fn importance_sampling_light(
//     &self,
//     world_position: Node<Vec3<f32>>,
//     sampler: &dyn DeviceSampler,
//   ) -> (RTLightSampling, Node<bool>) {
//     let s = (self as &dyn ShaderLightSource).importance_sampling_light(world_position, sampler);
//     (s, val(true))
//   }
// }

impl ShaderLightSource for ENode<PointLightShaderInfo> {
  fn radiant_power(&self) -> Node<Vec3<f32>> {
    val(4. * f32::PI()) * self.luminance_intensity
  }
  /// dirac distribution
  fn pdf(&self, _: Node<Vec3<f32>>) -> Node<f32> {
    val(0.)
  }

  fn importance_sampling_light_impl(
    &self,
    surface_position: Node<Vec3<f32>>,
    _: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    (self.position - surface_position).normalize()
  }

  /// this function should never be called other than importance_sampling_light, because it's
  /// dirac distributed, here the sample direction is ignored on propose
  fn compute_sample_radiance(
    &self,
    surface_position: Node<Vec3<f32>>,
    _: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    let sampling_dir = self.position - surface_position;
    let distance = sampling_dir.length();
    self.luminance_intensity / (distance * distance).splat()
  }

  /// override the default because it's dirac distributed
  fn importance_sampling_light(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let sampling_dir = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);

    RTLightSampling {
      sampling_dir,
      pdf: val(1.), // override because it's a dirac distribution
      radiance,
    }
  }
}
