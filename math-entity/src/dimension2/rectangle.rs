use crate::{ContainAble, HyperAABB};
use rendiation_math::Vector;

pub type Rectangle = HyperAABB<f32, 2>;

impl Rectangle {
  pub fn width(&self) -> f32 {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> f32 {
    self.max.y - self.min.y
  }
}

impl ContainAble<Vector<f32, 2>, 2> for Rectangle {
  fn contains(&self, v: &Vector<f32, 2>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}
