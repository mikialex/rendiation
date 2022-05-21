use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<U>(pub U);

impl<U> Point<U> {
  pub fn new(v: U) -> Self {
    Self(v)
  }
}

impl<T, U, V, M, const D: usize> SpaceEntity<T, D> for Point<U>
where
  T: Scalar,
  M: SquareMatrixDimension<D>,
  V: SpaceEntity<T, D, Matrix = M>,
  U: Positioned<Position = V>,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.0.mut_position().apply_matrix(mat);
    self
  }
}

impl<U: Copy> Point<U> {
  pub fn map<V>(&self, f: impl Fn(U) -> V) -> Point<V> {
    Point(f(self.0))
  }
}
