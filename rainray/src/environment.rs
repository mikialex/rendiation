use crate::math::Vec3;
use rendiation_algebra::Lerp;
use rendiation_geometry::Ray3;

pub trait Environment: Sync + 'static {
  fn sample(&self, ray: &Ray3) -> Vec3;
}

pub struct SolidEnvironment {
  pub intensity: Vec3,
}

impl SolidEnvironment {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

impl Environment for SolidEnvironment {
  fn sample(&self, _ray: &Ray3) -> Vec3 {
    self.intensity
  }
}

pub struct GradientEnvironment {
  pub top_intensity: Vec3,
  pub bottom_intensity: Vec3,
}

impl Environment for GradientEnvironment {
  fn sample(&self, ray: &Ray3) -> Vec3 {
    let t = ray.direction.y / 2.0 + 1.;
    self.bottom_intensity.lerp(self.top_intensity, t)
  }
}
