use crate::math::Vec3;
use rendiation_math::Lerp;
use rendiation_math_entity::Ray3;

pub trait Environment: Sync {
  fn sample(&self, ray: &Ray3) -> Vec3;
}

pub struct SolidEnvironment {
  pub intensity: Vec3,
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
