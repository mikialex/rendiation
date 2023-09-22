use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<U>(pub U);

impl<U> Point<U> {
  pub fn new(v: U) -> Self {
    Self(v)
  }
}

pub type PointPointIter<V> = impl Iterator<Item = V>;
impl<U> IntoIterator for Point<U> {
  type Item = U;
  type IntoIter = PointPointIter<U>;
  fn into_iter(self) -> PointPointIter<U> {
    std::iter::once(self.0)
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

impl<U> Point<U> {
  pub fn map<V>(self, mut f: impl FnMut(U) -> V) -> Point<V> {
    Point(f(self.0))
  }

  pub fn filter_map<V>(self, mut f: impl FnMut(U) -> Option<V>) -> Option<Point<V>> {
    Point(f(self.0)?).into()
  }
}
