use rendiation_math::*;

use crate::{SolidEntity, SpaceEntity};

pub struct HyperAABB<T: Scalar, const D: usize> {
  pub min: VectorType<T, D>,
  pub max: VectorType<T, D>,
}

impl<T: Scalar, const D: usize> SpaceEntity<D> for HyperAABB<T, D> {}
impl<T: Scalar, const D: usize> SolidEntity<D> for HyperAABB<T, D> {}

impl<T: Scalar, const D: usize> Copy for HyperAABB<T, D> where VectorType<T, D>: Copy {}

impl<T: Scalar, const D: usize> Clone for HyperAABB<T, D>
where
  VectorType<T, D>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      min: self.min.clone(),
      max: self.max.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperAABB<T, D> {
  pub fn new(min: VectorType<T, D>, max: VectorType<T, D>) -> Self {
    Self { min, max }
  }
}
