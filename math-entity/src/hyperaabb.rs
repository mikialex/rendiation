use rendiation_math::*;

use crate::{SolidEntity, SpaceEntity};

pub struct HyperAABB<T: Scalar, const D: usize> {
  pub min: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub max: <VectorMark<T> as DimensionalVec<T, D>>::Type,
}

impl<T: Scalar, const D: usize> SpaceEntity<D> for HyperAABB<T, D> {}
impl<T: Scalar, const D: usize> SolidEntity<D> for HyperAABB<T, D> {}

impl<T: Scalar, const D: usize> Copy for HyperAABB<T, D> where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy
{
}

impl<T: Scalar, const D: usize> Clone for HyperAABB<T, D>
where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      min: self.min.clone(),
      max: self.max.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperAABB<T, D> {
  pub fn new(
    min: <VectorMark<T> as DimensionalVec<T, D>>::Type,
    max: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  ) -> Self {
    Self { min, max }
  }
}
