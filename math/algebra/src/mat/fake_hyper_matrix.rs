use std::ops::*;

use num_traits::One;

use crate::{Scalar, SquareMatrix, SquareMatrixDimension};

#[derive(Copy, Clone)]
pub struct FakeHyperSquareMatrix<T, const D: usize>([T; D]);

impl<T: Scalar, const D: usize> One for FakeHyperSquareMatrix<T, D> {
  fn one() -> Self {
    unimplemented!()
  }
}
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

impl<T, const D: usize> Add<Self> for FakeHyperSquareMatrix<T, D> {
  type Output = Self;

  fn add(self, _rhs: Self) -> Self::Output {
    unreachable!()
  }
}
impl<T, const D: usize> Sub<Self> for FakeHyperSquareMatrix<T, D> {
  type Output = Self;

  fn sub(self, _rhs: Self) -> Self::Output {
    unreachable!()
  }
}
impl<T, const D: usize> Mul<Self> for FakeHyperSquareMatrix<T, D> {
  type Output = Self;

  fn mul(self, _rhs: Self) -> Self::Output {
    unreachable!()
  }
}
