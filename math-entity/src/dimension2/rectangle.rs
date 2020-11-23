use crate::{ContainAble, AABB};
use rendiation_math::Vec2;

pub type Rectangle = AABB<Vec2<f32>>;

impl Rectangle {
  pub fn width(&self) -> f32 {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> f32 {
    self.max.y - self.min.y
  }
}

impl ContainAble<Vec2<f32>> for Rectangle {
  fn contains(&self, v: &Vec2<f32>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}
