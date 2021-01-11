use crate::{Scalar, SquareMatrix, SquareMatrixDimension};

#[derive(Copy, Clone)]
pub struct FakeHyperSquareMatrix<T, const D: usize>([T; D]);
impl<T: Scalar, const D: usize> SquareMatrixDimension<D> for FakeHyperSquareMatrix<T, D> {}
impl<T: Scalar, const D: usize> SquareMatrix<T> for FakeHyperSquareMatrix<T, D> {
  fn transpose(&self) -> Self {
    unimplemented!()
  }
  fn inverse(&self) -> Option<Self> {
    unimplemented!()
  }
  fn det(&self) -> T {
    unimplemented!()
  }

  fn identity() -> Self {
    unimplemented!()
  }
}
