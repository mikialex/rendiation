use rendiation_math::*;

use crate::SpaceEntity;

pub struct HyperPlane<T: Scalar, const D: usize> {
  pub normal: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub constant: T,
}

impl<T: Scalar, const D: usize> SpaceEntity<D> for HyperPlane<T, D> {}

impl<T: Scalar, const D: usize> Copy for HyperPlane<T, D> where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy
{
}

impl<T: Scalar, const D: usize> Clone for HyperPlane<T, D>
where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      normal: self.normal.clone(),
      constant: self.constant.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperPlane<T, D> {
  pub fn new(normal: <VectorMark<T> as DimensionalVec<T, D>>::Type, constant: T) -> Self {
    Self { normal, constant }
  }
}
