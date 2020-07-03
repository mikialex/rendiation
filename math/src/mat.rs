use crate::{Mat3, Mat2, Mat4};

pub trait Matrix {}

pub trait SquareMatrix: Matrix {}

impl<T> Matrix for Mat2<T>{}
impl<T> Matrix for Mat3<T>{}
impl<T> Matrix for Mat4<T>{}

impl<T> SquareMatrix for Mat2<T>{}
impl<T> SquareMatrix for Mat3<T>{}
impl<T> SquareMatrix for Mat4<T>{}

pub struct ColumMajor<M: SquareMatrix> {
  pub mat: M,
}

pub struct RawMajor<M: SquareMatrix> {
  pub mat: M,
}
