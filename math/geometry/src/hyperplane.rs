use rendiation_algebra::*;

use crate::SpaceEntity;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct HyperPlane<T: Scalar, V> {
  pub normal: NormalizedVector<T, V>,
  pub constant: T,
}

impl<T: Scalar, const D: usize, V> SpaceEntity<T, D> for HyperPlane<T, V> {
  default fn apply_matrix(&mut self, _mat: SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

impl<T: Scalar, V> HyperPlane<T, V> {
  pub fn new(normal: NormalizedVector<T, V>, constant: T) -> Self {
    Self { normal, constant }
  }
}
