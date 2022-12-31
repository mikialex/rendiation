use crate::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection<T: Scalar>: Send + Sync {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<T>);

  fn create_projection<S: NDCSpaceMapper>(&self) -> Mat4<T> {
    let mut projection = Mat4::default();
    self.update_projection::<S>(&mut projection);
    projection
  }

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: T, view_height: T) -> T;

  fn project<S: NDCSpaceMapper>(&self, point: Vec3<T>) -> Vec3<T> {
    (self.create_projection::<S>() * point.expand_with_one()).xyz()
  }
  fn un_project<S: NDCSpaceMapper>(&self, point: Vec3<T>) -> Vec3<T> {
    (self.create_projection::<S>().inverse_or_identity() * point.expand_with_one()).xyz()
  }
}

pub trait ResizableProjection<T: Scalar>: Projection<T> {
  fn resize(&mut self, size: (T, T));
}
