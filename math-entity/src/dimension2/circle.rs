use crate::{ContainAble, HyperSphere, Rectangle, SpaceBounding};
use rendiation_math::{Vec2, Vector};

pub type Circle = HyperSphere<f32, 2>;

impl ContainAble<Vector<f32, 2>, 2> for Circle {
  fn contains(&self, v: &Vector<f32, 2>) -> bool {
    (*v - self.center).length2() <= self.radius * self.radius
  }
}

impl SpaceBounding<Rectangle> for Circle {
  fn to_bounding(&self) -> Rectangle {
    Rectangle {
      min: (self.center.data - Vec2::splat(self.radius)).into(),
      max: (self.center.data + Vec2::splat(self.radius)).into(),
    }
  }
}
