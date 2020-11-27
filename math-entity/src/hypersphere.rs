use rendiation_math::*;

use crate::{SolidEntity, SpaceEntity};

pub struct HyperSphere<T: Scalar, const D: usize> {
  pub center: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub radius: T,
}

impl<T: Scalar, const D: usize> SpaceEntity<D> for HyperSphere<T, D> {}
impl<T: Scalar, const D: usize> SolidEntity<D> for HyperSphere<T, D> {}

impl<T: Scalar, const D: usize> Copy for HyperSphere<T, D> where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy
{
}

impl<T: Scalar, const D: usize> Clone for HyperSphere<T, D>
where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      center: self.center.clone(),
      radius: self.radius.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperSphere<T, D> {
  pub fn new(center: <VectorMark<T> as DimensionalVec<T, D>>::Type, radius: T) -> Self {
    Self { center, radius }
  }
}
