use num_traits::One;

use crate::Scalar;

pub trait SquareMatrixDimension<const D: usize>: Copy {}

pub trait SquareMatrix<T: Scalar>: Sized + One {
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

  #[must_use]
  fn max_scale(&self) -> T;
}
