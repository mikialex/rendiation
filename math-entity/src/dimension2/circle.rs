use crate::{HyperSphere, LebesgueMeasurable, Rectangle, SpaceBounding};
use rendiation_math::*;

pub type Circle = HyperSphere<f32, 2>;

impl SpaceBounding<f32, Rectangle, 2> for Circle {
  fn to_bounding(&self) -> Rectangle {
    Rectangle {
      min: self.center - Vec2::splat(self.radius),
      max: self.center + Vec2::splat(self.radius),
    }
  }
}

impl LebesgueMeasurable<f32, 2> for Circle {
  #[inline(always)]
  fn measure(&self) -> f32 {
    std::f32::consts::PI * self.radius * self.radius
  }
}

impl LebesgueMeasurable<f32, 1> for Circle {
  #[inline(always)]
  fn measure(&self) -> f32 {
    std::f32::consts::PI * self.radius * 2.
  }
}
