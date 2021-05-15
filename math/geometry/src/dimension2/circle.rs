use crate::{HyperSphere, LebesgueMeasurable};
use rendiation_algebra::*;

pub type Circle<T = f32> = HyperSphere<T, Vec2<T>>;

impl<T: Scalar> LebesgueMeasurable<T, 2> for Circle<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::PI() * self.radius * self.radius
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 1> for Circle<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::PI() * self.radius * T::two()
  }
}
