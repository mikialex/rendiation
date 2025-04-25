use crate::*;

mod perspective;
pub use perspective::*;
mod orth;
pub use orth::*;

pub trait Projection<T: Scalar>: Send + Sync {
  fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T>;

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: T, view_height_in_pixel: T) -> T;

  fn project(&self, point: Vec3<T>, mapper: &dyn NDCSpaceMapper<T>) -> Vec3<T> {
    (self.compute_projection_mat(mapper) * point.expand_with_one()).xyz()
  }
  fn un_project(&self, point: Vec3<T>, mapper: &dyn NDCSpaceMapper<T>) -> Vec3<T> {
    (self.compute_projection_mat(mapper).inverse_or_identity() * point.expand_with_one()).xyz()
  }
}

pub trait ResizableProjection<T: Scalar>: Projection<T> {
  fn resize(&mut self, size: (T, T));
}

impl<T: Scalar> Mat4<T> {
  /// check if the mat is the perspective, assume the mat is the common projection(perspective or orthographic)
  pub fn check_is_perspective_matrix_assume_common_projection(&self) -> bool {
    self.c4 == -T::one()
  }

  /// get the near and far assume the mat is the common projection(perspective or orthographic)
  pub fn get_near_far_assume_is_common_projection(&self) -> (T, T) {
    if self.check_is_perspective_matrix_assume_common_projection() {
      self.get_near_far_assume_perspective()
    } else {
      self.get_near_far_assume_orthographic()
    }
  }
}
