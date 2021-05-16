use rendiation_algebra::Vec3;
use rendiation_algebra::Vector;

pub trait Background: 'static {}

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

pub struct GradientBackground {
  pub top_intensity: Vec3<f32>,
  pub bottom_intensity: Vec3<f32>,
}
