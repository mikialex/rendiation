use crate::{HyperSphere, LebesgueMeasurable, Rectangle, SpaceBounding};
use rendiation_math::*;

pub type Circle<T = f32> = HyperSphere<T, 2>;

impl<T: Scalar> SpaceBounding<T, Rectangle<T>, 2> for Circle<T> {
  fn to_bounding(&self) -> Rectangle<T> {
    Rectangle {
      min: self.center - Vec2::splat(self.radius),
      max: self.center + Vec2::splat(self.radius),
    }
  }
}

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
