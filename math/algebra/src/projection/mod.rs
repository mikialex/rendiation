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
