pub mod box2;
pub mod circle;
pub use box2::*;
pub use circle::*;
use rendiation_math::Vec2;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}

pub trait Positioned2D: Copy {
  fn position(&self) -> Vec2<f32>;
}

impl Positioned2D for Vec2<f32> {
  fn position(&self) -> Vec2<f32> {
    *self
  }
}