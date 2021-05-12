use crate::SpaceEntity;
use rendiation_algebra::{Scalar, SquareMatrixType};
use std::ops::DerefMut;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<U>(pub U);

impl<U> Point<U> {
  pub fn new(v: U) -> Self {
    Self(v)
  }
}

impl<T, U, const D: usize, V> SpaceEntity<T, D> for Point<U>
where
  T: Scalar,
  V: SpaceEntity<T, D>,
  U: DerefMut<Target = V>,
{
  fn apply_matrix(&mut self, mat: SquareMatrixType<T, D>) -> &mut Self {
    self.0.deref_mut().apply_matrix(mat);
    self
  }
}

impl<U: Copy> Point<U> {
  pub fn map<V>(&self, f: impl Fn(U) -> V) -> Point<V> {
    Point(f(self.0))
  }
}
