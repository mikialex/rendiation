use rendiation_math::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point3<T>(pub T);

impl<T: Copy> Point3<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

pub trait PositionedPoint3: Copy {
  fn position(&self) -> Vec3<f32>;
}

impl PositionedPoint3 for Vec3<f32> {
  fn position(&self) -> Vec3<f32> {
    *self
  }
}
