use rendiation_algebra::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct HyperRay<T: Scalar, V> {
  pub origin: V,
  pub direction: NormalizedVector<T, V>,
}

impl<T: Scalar, V: VectorSpace<T>> HyperRay<T, V> {
  pub fn at(&self, distance: T) -> V {
    self.origin + self.direction * distance
  }
}

impl<T: Scalar, const D: usize, V> SpaceEntity<T, D> for HyperRay<T, V> {
  default fn apply_matrix(&mut self, _mat: SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

impl<T: Scalar, V> HyperRay<T, V> {
  pub fn new(origin: V, direction: NormalizedVector<T, V>) -> Self {
    HyperRay { origin, direction }
  }
}
