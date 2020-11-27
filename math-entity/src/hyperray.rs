use rendiation_math::*;

pub struct HyperRay<T, const D: usize> {
  pub origin: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub direction: <VectorMark<T> as DimensionalVec<T, D>>::Type,
}

impl<T, const D: usize> Copy for HyperRay<T, D>
where
  T: Copy,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy,
{
}

impl<T, const D: usize> Clone for HyperRay<T, D>
where
  T: Clone,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      origin: self.origin.clone(),
      direction: self.direction.clone(),
    }
  }
}

impl<T, const D: usize> HyperRay<T, D> {
  pub fn new(
    origin: <VectorMark<T> as DimensionalVec<T, D>>::Type,
    direction: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  ) -> Self {
    HyperRay { origin, direction }
  }
}
