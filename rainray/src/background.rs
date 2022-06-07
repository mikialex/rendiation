use rendiation_algebra::*;
use rendiation_geometry::Ray3;

pub trait Background: Send + Sync + 'static + dyn_clone::DynClone {
  fn sample(&self, ray: &Ray3) -> Vec3<f32>;
}

dyn_clone::clone_trait_object!(Background);

pub trait BackgroundToBoxed: Background + Sized {
  fn to_boxed(self) -> Box<dyn Background> {
    Box::new(self) as Box<dyn Background>
  }
}

impl Background for SolidBackground {
  fn sample(&self, _ray: &Ray3) -> Vec3<f32> {
    self.intensity
  }
}

impl BackgroundToBoxed for GradientBackground {}
impl Background for GradientBackground {
  fn sample(&self, ray: &Ray3) -> Vec3<f32> {
    let t = ray.direction.y / 2.0 + 1.;
    self.bottom_intensity.lerp(self.top_intensity, t)
  }
}

#[derive(Clone)]
pub struct SolidBackground {
  pub intensity: Vec3<f32>,
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      intensity: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl SolidBackground {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

#[derive(Clone)]
pub struct GradientBackground {
  pub top_intensity: Vec3<f32>,
  pub bottom_intensity: Vec3<f32>,
}
