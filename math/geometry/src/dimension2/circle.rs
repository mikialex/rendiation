use crate::{HyperSphere, LebesgueMeasurable, SolidEntity};
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

impl SolidEntity<f32, 2> for Circle {
  type Center = Vec2<f32>;
  fn centroid(&self) -> Self::Center {
    self.center
  }
}
