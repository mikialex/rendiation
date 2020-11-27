use crate::{ContainAble, HyperSphere, Rectangle, SpaceBounding};
use rendiation_math::*;

pub type Circle = HyperSphere<f32, 2>;

impl ContainAble<Vec2<f32>, 2> for Circle {
  fn contains(&self, v: &Vec2<f32>) -> bool {
    (*v - self.center).length2() <= self.radius * self.radius
  }
}

impl SpaceBounding<Rectangle, 2> for Circle {
  fn to_bounding(&self) -> Rectangle {
    Rectangle {
      min: (self.center - Vec2::splat(self.radius)).into(),
      max: (self.center + Vec2::splat(self.radius)).into(),
    }
  }
}
