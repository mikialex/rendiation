use crate::{ContainAble, HyperAABB, LebesgueMeasurable};
use rendiation_algebra::{Scalar, SpaceEntity, SquareMatrixType, Vec2};

pub type Rectangle<T = f32> = HyperAABB<Vec2<T>>;

impl<T: Scalar> Rectangle<T> {
  pub fn width(&self) -> T {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> T {
    self.max.y - self.min.y
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 2> for Rectangle<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    self.width() * self.height()
  }
}

impl<T: Scalar> SpaceEntity<T, 2> for Rectangle<T> {
  #[inline(always)]
  fn apply_matrix(&mut self, _mat: SquareMatrixType<T, 2>) -> &mut Self {
    todo!()
  }
}

impl<T: Scalar> ContainAble<T, Vec2<T>, 2> for Rectangle<T> {
  fn contains(&self, v: &Vec2<T>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}
