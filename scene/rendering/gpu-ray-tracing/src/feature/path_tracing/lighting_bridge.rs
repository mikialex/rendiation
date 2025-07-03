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
  pub distance: Node<f32>,
  pub pdf: Node<f32>,
  pub radiance: Node<Vec3<f32>>,
}

pub trait ShaderLightSource {
  /// in watt
  fn radiant_power(&self) -> Node<Vec3<f32>>;

  /// for dirac distribution, return 0
  fn pdf(&self, sample_dir: Node<Vec3<f32>>) -> Node<f32>;

  /// return (direction, distance)
  /// todo, should we just return direction??
  fn importance_sampling_light_impl(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (Node<Vec3<f32>>, Node<f32>);

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
    let (sampling_dir, distance) = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);
    let pdf = self.pdf(sampling_dir);
    RTLightSampling {
      sampling_dir,
      pdf,
      radiance,
      distance,
    }
  }
}

impl DevicePathTracingLightingInvocation for PointLightStorageShaderAPIInstance {
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    let s = (self as &dyn ShaderLightSource).importance_sampling_light(world_position, sampler);
    (s, val(true))
  }
}

impl ShaderLightSource for PointLightStorageShaderAPIInstance {
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
  ) -> (Node<Vec3<f32>>, Node<f32>) {
    let position_to_light = self.position.expand().f1 - surface_position;
    let distance = position_to_light.length();
    (position_to_light / distance.splat(), distance)
  }

  /// this function should never be called other than importance_sampling_light, because it's
  /// dirac distributed, here the sample direction is ignored on propose
  fn compute_sample_radiance(
    &self,
    surface_position: Node<Vec3<f32>>,
    _: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    let sampling_dir = self.position.expand().f1 - surface_position;
    let distance = sampling_dir.length();
    self.luminance_intensity / (distance * distance).splat()
  }

  /// override the default because it's dirac distributed
  fn importance_sampling_light(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let (sampling_dir, distance) = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);

    RTLightSampling {
      sampling_dir,
      pdf: val(1.), // override because it's a dirac distribution
      radiance,
      distance,
    }
  }
}

impl DevicePathTracingLightingInvocation for SpotLightStorageShaderAPIInstance {
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    let s = (self as &dyn ShaderLightSource).importance_sampling_light(world_position, sampler);
    (s, val(true))
  }
}

impl ShaderLightSource for SpotLightStorageShaderAPIInstance {
  /// the spot light falloff fn is smoothstep(a polynomial), which is integrable
  fn radiant_power(&self) -> Node<Vec3<f32>> {
    let cos_falloff_start = self.half_penumbra_cos;
    let cos_falloff_end = self.half_cone_cos;
    let ratio = (val(1.) - cos_falloff_start) + (cos_falloff_start - cos_falloff_end) / val(2.);
    val(2. * f32::PI()) * self.luminance_intensity * ratio
  }

  /// dirac distribution
  fn pdf(&self, _: Node<Vec3<f32>>) -> Node<f32> {
    val(0.)
  }

  fn importance_sampling_light_impl(
    &self,
    surface_position: Node<Vec3<f32>>,
    _: &dyn DeviceSampler,
  ) -> (Node<Vec3<f32>>, Node<f32>) {
    let position_to_light = self.position.expand().f1 - surface_position;
    let distance = position_to_light.length();
    (position_to_light / distance.splat(), distance)
  }

  /// this function should never be called other than importance_sampling_light, because it's
  /// dirac distributed, here the sample direction is ignored on propose
  fn compute_sample_radiance(
    &self,
    surface_position: Node<Vec3<f32>>,
    _: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    let sampling_dir = self.position.expand().f1 - surface_position;
    let distance = sampling_dir.length();
    self.luminance_intensity / (distance * distance).splat()
  }

  /// override the default because it's dirac distributed
  fn importance_sampling_light(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let (sampling_dir, distance) = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);

    RTLightSampling {
      sampling_dir,
      pdf: val(1.), // override because it's a dirac distribution
      radiance,
      distance,
    }
  }
}

impl DevicePathTracingLightingInvocation for DirectionalLightStorageShaderAPIInstance {
  fn importance_sampling_light(
    &self,
    world_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> (RTLightSampling, Node<bool>) {
    let s = (self as &dyn ShaderLightSource).importance_sampling_light(world_position, sampler);
    (s, val(true))
  }
}

impl ShaderLightSource for DirectionalLightStorageShaderAPIInstance {
  /// the directional light power should equals the entire scene's projection area * luminance.
  /// The projection area is hard to compute. One common estimation is to use the scene's
  /// bounding's projection.
  ///
  /// todo, for now we hard code this area.
  fn radiant_power(&self) -> Node<Vec3<f32>> {
    let area_estimation = val(10. * 10.);
    area_estimation * self.illuminance
  }
  /// dirac distribution
  fn pdf(&self, _: Node<Vec3<f32>>) -> Node<f32> {
    val(0.)
  }

  fn importance_sampling_light_impl(
    &self,
    _: Node<Vec3<f32>>,
    _: &dyn DeviceSampler,
  ) -> (Node<Vec3<f32>>, Node<f32>) {
    (-self.direction, val(f32::MAX))
  }

  /// this function should never be called other than importance_sampling_light, because it's
  /// dirac distributed, here the sample direction is ignored on propose
  fn compute_sample_radiance(&self, _: Node<Vec3<f32>>, _: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    self.illuminance
  }

  /// override the default because it's dirac distributed
  fn importance_sampling_light(
    &self,
    surface_position: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTLightSampling {
    let (sampling_dir, distance) = self.importance_sampling_light_impl(surface_position, sampler);
    let radiance = self.compute_sample_radiance(surface_position, sampling_dir);

    RTLightSampling {
      sampling_dir,
      pdf: val(1.), // override because it's a dirac distribution
      radiance,
      distance,
    }
  }
}
