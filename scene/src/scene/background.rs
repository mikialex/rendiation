use crate::RAL;
use rendiation_algebra::Vec3;

pub trait Background<T: RAL> {
  fn render(&self, renderer: &mut T::Renderer, builder: T::RenderTarget);
}

pub struct SolidBackground {
  pub color: Vec3<f32>,
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self::new()
  }
}

impl SolidBackground {
  pub fn new() -> Self {
    Self {
      color: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}
