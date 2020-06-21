use rendiation_math::Vec3;
use crate::SceneGraphBackend;

pub trait Background<T: SceneGraphBackend> {
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
