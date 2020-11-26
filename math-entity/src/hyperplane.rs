use rendiation_math::*;

use crate::SpaceEntity;

pub struct HyperPlane<T, const D: usize> {
  pub normal: Vector<T, D>,
  pub constant: T,
}

impl<T, const D: usize> SpaceEntity<D> for HyperPlane<T, D> {}

impl<T, const D: usize> Copy for HyperPlane<T, D>
where
  T: Copy,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy,
{
}

impl<T, const D: usize> Clone for HyperPlane<T, D>
where
  T: Clone,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      normal: self.normal.clone(),
      constant: self.constant.clone(),
    }
  }
}

impl<T, const D: usize> HyperPlane<T, D> {
  pub fn new(normal: Vector<T, D>, constant: T) -> Self {
    Self { normal, constant }
  }
}
