use crate::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection<T: Scalar>: Send + Sync {
  fn compute_projection_mat<S: NDCSpaceMapper<T>>(&self) -> Mat4<T>;

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: T, view_height: T) -> T;

  fn project<S: NDCSpaceMapper<T>>(&self, point: Vec3<T>) -> Vec3<T> {
    (self.compute_projection_mat::<S>() * point.expand_with_one()).xyz()
  }
  fn un_project<S: NDCSpaceMapper<T>>(&self, point: Vec3<T>) -> Vec3<T> {
    (self.compute_projection_mat::<S>().inverse_or_identity() * point.expand_with_one()).xyz()
  }
}

pub trait ResizableProjection<T: Scalar>: Projection<T> {
  fn resize(&mut self, size: (T, T));
}
