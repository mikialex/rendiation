use std::marker::PhantomData;

use crate::{FakeHyperSquareMatrix, Mat3, Mat4, Scalar};

pub trait SquareMatrixDimension<const D: usize> {}
pub trait SquareMatrixImpl {}

pub trait SquareMatrix<T: Scalar> {}

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
