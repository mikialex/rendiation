use num_traits::One;

use crate::Scalar;

/// Mark the dimension of the space that the matrix is the homogeneous transform of. For example
/// [`Mat4`](crate::Mat4) is `SquareMatrixDimension<3>` because it transforms the 3d space.
pub trait SquareMatrixDimension<const D: usize>: Copy {}

/// The common operations of square matrix.
pub trait SquareMatrix<T: Scalar>: Sized + One {
  #[must_use]
  fn identity() -> Self;

  #[must_use]
  fn transpose(&self) -> Self;

  /// Return None if the matrix is singular (the determinant is exactly zero). Note the nearly
  /// singular matrix still has inverse but the result may be inaccurate.
  #[must_use]
  fn inverse(&self) -> Option<Self>;

  #[must_use]
  fn inverse_or_identity(&self) -> Self {
    self.inverse().unwrap_or_else(|| Self::identity())
  }

  /// The determinant.
  #[must_use]
  fn det(&self) -> T;

  /// The max scale factor (the max length of the transformed axis) of the linear part of the
  /// transform in the space this matrix applies to. For example for [`Mat4`](crate::Mat4) only
  /// the upper left 3x3 is considered. This is useful to transform the radius of sphere.
  #[must_use]
  fn max_scale(&self) -> T;
}
