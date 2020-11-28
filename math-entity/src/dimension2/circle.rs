use crate::{HyperSphere, Rectangle, SpaceBounding};
use rendiation_math::*;

pub type Circle = HyperSphere<f32, 2>;

impl SpaceBounding<Rectangle, 2> for Circle {
  fn to_bounding(&self) -> Rectangle {
    Rectangle {
      min: (self.center - Vec2::splat(self.radius)).into(),
      max: (self.center + Vec2::splat(self.radius)).into(),
    }
  }
}
