use crate::RALBackend;
use rendiation_math::Vec3;

pub trait Background<T: RALBackend> {
  fn render(&self, renderer: &mut T::Renderer, builder: T::RenderTarget);
}

pub struct SolidBackground {
  pub color: Vec3<f32>,
}

impl SolidBackground {
  pub fn new() -> Self {
    Self {
      color: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}
