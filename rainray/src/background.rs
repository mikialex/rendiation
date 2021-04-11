use crate::math::Vec3;
use rendiation_algebra::Lerp;
use rendiation_geometry::Ray3;
use sceno::{GradientBackground, SolidBackground};

pub trait Background: Sync + 'static {
  fn sample(&self, ray: &Ray3) -> Vec3;
}
pub trait BackgroundToBoxed: Background + Sized {
  fn to_boxed(self) -> Box<dyn Background> {
    Box::new(self) as Box<dyn Background>
  }
}

impl Background for SolidBackground {
  fn sample(&self, _ray: &Ray3) -> Vec3 {
    self.intensity
  }
}

impl BackgroundToBoxed for GradientBackground {}
impl Background for GradientBackground {
  fn sample(&self, ray: &Ray3) -> Vec3 {
    let t = ray.direction.y / 2.0 + 1.;
    self.bottom_intensity.lerp(self.top_intensity, t)
  }
}
