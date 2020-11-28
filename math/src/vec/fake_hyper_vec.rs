use std::ops::*;

use crate::{Scalar, Vector, VectorDimension, VectorImpl};

#[derive(Copy, Clone)]
pub struct FakeHyperVec<T, const D: usize>([T; D]);
impl<T: Scalar, const D: usize> VectorDimension<D> for FakeHyperVec<T, D> {}
impl<T: Scalar, const D: usize> VectorImpl for FakeHyperVec<T, D> {}
impl<T, const D: usize> Add<Self> for FakeHyperVec<T, D> {
  type Output = Self;

  fn add(self, _rhs: Self) -> Self::Output {
    unreachable!()
  }
}
impl<T, const D: usize> Sub<Self> for FakeHyperVec<T, D> {
  type Output = Self;

  fn sub(self, _rhs: Self) -> Self::Output {
    unreachable!()
  }
}
impl<T, const D: usize> Mul<T> for FakeHyperVec<T, D> {
  type Output = Self;

  fn mul(self, _rhs: T) -> Self::Output {
    unreachable!()
  }
}
impl<T: Scalar, const D: usize> Vector<T> for FakeHyperVec<T, D> {
  fn dot(&self, _b: Self) -> T {
    unreachable!()
  }

  fn cross(&self, _b: Self) -> Self {
    unreachable!()
  }
}
