pub trait Matrix {}

pub trait SquareMatrix: Matrix {}

pub struct ColumMajor<M: SquareMatrix> {
  pub mat: M,
}

pub struct RawMajor<M: SquareMatrix> {
  pub mat: M,
}
