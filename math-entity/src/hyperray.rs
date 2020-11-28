use rendiation_math::*;

pub struct HyperRay<T: Scalar, const D: usize> {
  pub origin: VectorType<T, D>,
  pub direction: VectorType<T, D>,
}

impl<T: Scalar, const D: usize> Copy for HyperRay<T, D> where VectorType<T, D>: Copy {}

impl<T: Scalar, const D: usize> Clone for HyperRay<T, D>
where
  VectorType<T, D>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      origin: self.origin.clone(),
      direction: self.direction.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperRay<T, D> {
  pub fn new(origin: VectorType<T, D>, direction: VectorType<T, D>) -> Self {
    HyperRay { origin, direction }
  }
}
