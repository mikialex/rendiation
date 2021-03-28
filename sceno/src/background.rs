use rendiation_algebra::Vec3;

pub trait Background {}

pub struct SolidBackground {
  pub color: Vec3<f32>,
}

impl Background for SolidBackground {}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      color: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}
