use rendiation_math::{Scalar, SquareMatrixType, VectorType};

use crate::{Positioned, SpaceEntity};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<V>(pub V);

impl<V: Copy> Point<V> {
  pub fn new(v: V) -> Self {
    Self(v)
  }
}

impl<T: Scalar, V: Positioned<T, D>, const D: usize> SpaceEntity<T, D> for Point<V> {
  fn apply_matrix(&mut self, mat: SquareMatrixType<T, D>) -> &mut Self {
    self.0.position_mut().apply_matrix(mat);
    self
  }
}

impl<V> Point<V> {
  pub fn map_position<T, const D: usize>(&self) -> Point<VectorType<T, D>>
  where
    T: Scalar,
    V: Positioned<T, D>,
  {
    self.map(|p| p.position())
  }
}

impl<V: Copy> Point<V> {
  pub fn map<U>(&self, f: impl Fn(V) -> U) -> Point<U> {
    Point(f(self.0))
  }
}
