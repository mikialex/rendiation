use rendiation_math::*;

pub struct HyperRay<T: Scalar, const D: usize> {
  pub origin: VectorType<T, D>,
  pub direction: NormalizedVector<T, VectorType<T, D>>,
}

impl<T: Scalar, const D: usize> HyperRay<T, D> {
  pub fn at(&self, distance: T) -> VectorType<T, D> {
    self.origin + self.direction * distance
  }
}

impl<T: Scalar, const D: usize> SpaceEntity<T, D> for HyperRay<T, D> {
  default fn apply_matrix(&mut self, _mat: SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

impl<T: Scalar, const D: usize> Copy for HyperRay<T, D> where VectorType<T, D>: Copy {}

impl<T: Scalar, const D: usize> Clone for HyperRay<T, D>
where
  VectorType<T, D>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      origin: self.origin,
      direction: self.direction,
    }
  }
}

impl<T: Scalar, const D: usize> HyperRay<T, D> {
  pub fn new(origin: VectorType<T, D>, direction: NormalizedVector<T, VectorType<T, D>>) -> Self {
    HyperRay { origin, direction }
  }
}
