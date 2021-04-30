use std::marker::PhantomData;

use crate::{FakeHyperSquareMatrix, Mat3, Mat4, Scalar};

pub trait SquareMatrixDimension<const D: usize> {}
pub trait SquareMatrixImpl {}

pub trait SquareMatrix<T: Scalar>: Sized {
  #[must_use]
  fn identity() -> Self;

  #[must_use]
  fn transpose(&self) -> Self;

  #[must_use]
  fn inverse(&self) -> Option<Self>;

  #[must_use]
  fn inverse_or_identity(&self) -> Self {
    self.inverse().unwrap_or(Self::identity())
  }

  #[must_use]
  fn det(&self) -> T;
}

pub trait DimensionalSquareMatrix<T: Scalar, const D: usize> {
  type Type: SquareMatrix<T> + SquareMatrixDimension<D> + Copy;
}

pub struct SquareMatrixMark<T>(PhantomData<T>);

impl<T: Scalar> DimensionalSquareMatrix<T, 2> for SquareMatrixMark<T> {
  type Type = Mat3<T>;
}
impl<T: Scalar> DimensionalSquareMatrix<T, 3> for SquareMatrixMark<T> {
  type Type = Mat4<T>;
}

impl<T: Scalar, const D: usize> DimensionalSquareMatrix<T, D> for SquareMatrixMark<T> {
  default type Type = FakeHyperSquareMatrix<T, D>;
}

pub type SquareMatrixType<T, const D: usize> =
  <SquareMatrixMark<T> as DimensionalSquareMatrix<T, D>>::Type;
