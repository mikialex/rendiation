use crate::{ContainAble, HyperAABB};
use rendiation_math::Vec2;

pub type Rectangle = HyperAABB<f32, 2>;

impl Rectangle {
  pub fn width(&self) -> f32 {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> f32 {
    self.max.y - self.min.y
  }
}

impl ContainAble<f32, Vec2<f32>, 2> for Rectangle {
  fn contains(&self, v: &Vec2<f32>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}
