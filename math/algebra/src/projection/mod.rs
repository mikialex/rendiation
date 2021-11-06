use crate::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection: Send + Sync {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>);

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;

  fn project(&self, point: Vec3<f32>) -> Vec3<f32> {
    todo!();
  }
  fn un_project(&self, point: Vec3<f32>) -> Vec3<f32> {
    todo!();
  }
}

pub trait ResizableProjection: Projection {
  fn resize(&mut self, size: (f32, f32));
}
