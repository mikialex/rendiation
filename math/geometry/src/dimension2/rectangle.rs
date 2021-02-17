use crate::{ContainAble, HyperAABB};
use rendiation_algebra::{Scalar, Vec2};

pub type Rectangle<T = f32> = HyperAABB<T, 2>;

impl<T: Scalar> Rectangle<T> {
  pub fn width(&self) -> T {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> T {
    self.max.y - self.min.y
  }
}

impl<T: Scalar> ContainAble<T, Vec2<T>, 2> for Rectangle<T> {
  fn contains(&self, v: &Vec2<T>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}
