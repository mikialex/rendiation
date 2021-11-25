use crate::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection: Send + Sync {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>);

  fn create_projection<S: NDCSpaceMapper>(&self) -> Mat4<f32> {
    let mut projection = Mat4::default();
    self.update_projection::<S>(&mut projection);
    projection
  }

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;

  fn project<S: NDCSpaceMapper>(&self, point: Vec3<f32>) -> Vec3<f32> {
    let point = Vec4::new(point.x, point.y, point.z, 1.);
    (point * self.create_projection::<S>()).xyz()
  }
  fn un_project<S: NDCSpaceMapper>(&self, point: Vec3<f32>) -> Vec3<f32> {
    let point = Vec4::new(point.x, point.y, point.z, 1.);
    (point * self.create_projection::<S>().inverse_or_identity()).xyz()
  }
}

pub trait ResizableProjection: Projection {
  fn resize(&mut self, size: (f32, f32));
}
