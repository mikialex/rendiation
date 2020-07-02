use std::ops::Deref;

pub trait Matrix {}

pub trait SquareMatrix: Matrix {}

pub struct ColumMajor<M: SquareMatrix> {
  mat: M,
}

pub struct RawMajor<M: SquareMatrix> {
  mat: M,
}
