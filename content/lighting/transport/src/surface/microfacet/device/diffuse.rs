use rendiation_shader_library::sampling::*;

use crate::*;

pub trait ShaderPhysicalDiffuse {
  fn albedo(&self) -> Node<Vec3<f32>>;
}

#[derive(Clone)]
pub struct ShaderLambertian {
  pub albedo: Node<Vec3<f32>>,
}
impl ShaderPhysicalDiffuse for ShaderLambertian {
  fn albedo(&self) -> Node<Vec3<f32>> {
    self.albedo
  }
}

impl ShaderLightTransportSurface for ShaderLambertian {
  fn bsdf(
    &self,
    _: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    // the light below the surface has no contribution
    light_dir
      .dot(normal)
      .greater_than(0.)
      .select(self.albedo / val(Vec3::splat(PI)), val(Vec3::zero()))
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    // // Simple cosine-sampling using Malley's method
    let sample = concentric_sample_disk_device_fn(sampler.next_2d());
    let x = sample.x();
    let y = sample.y();
    let z = (val(1.0) - x * x - y * y).max(0.).sqrt();
    tbn_fn(normal) * (x, y, z).into()
  }

  fn pdf(
    &self,
    _: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    light_dir.dot(normal).max(0.0) * val(INV_PI)
  }
}
