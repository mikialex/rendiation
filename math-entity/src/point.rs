use rendiation_math::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T>(pub T);

impl<T: Copy> Point<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

pub trait PositionedPoint: Copy {
  fn position(&self) -> Vec3<f32>;
}

impl PositionedPoint for Vec3<f32> {
  fn position(&self) -> Vec3<f32> {
    *self
  }
}
