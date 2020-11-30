use rendiation_math::*;

use crate::SpaceEntity;

pub struct HyperPlane<T: Scalar, const D: usize> {
  pub normal: VectorType<T, D>,
  pub constant: T,
}

impl<T: Scalar, const D: usize> SpaceEntity<T, D> for HyperPlane<T, D> {
  default fn apply_matrix(&mut self, _mat: &SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

impl<T: Scalar, const D: usize> Copy for HyperPlane<T, D> where VectorType<T, D>: Copy {}

impl<T: Scalar, const D: usize> Clone for HyperPlane<T, D>
where
  VectorType<T, D>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      normal: self.normal.clone(),
      constant: self.constant.clone(),
    }
  }
}

impl<T: Scalar, const D: usize> HyperPlane<T, D> {
  pub fn new(normal: VectorType<T, D>, constant: T) -> Self {
    Self { normal, constant }
  }
}
