use crate::transformed_object::TransformedObject;
use rendiation_math::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub struct CameraData {
  projection_matrix: Mat4<f32>,
  world_matrix: Mat4<f32>,
  view_matrix: Mat4<f32>,
  projection_changed: bool,
}

impl CameraData {
  pub fn update(&mut self, projection: impl Projection) {
    projection.update(&mut self.projection_matrix);
  }
}

pub trait Projection {
  fn update(&self, projection: &mut Mat4<f32>);
}

pub trait ResizableProjection {
  fn resize(&mut self, size: (f32, f32));
}

pub trait Camera: TransformedObject {
  fn update_projection(&mut self);
  fn get_projection_matrix(&self) -> &Mat4<f32>;

  fn get_vp_matrix(&self) -> Mat4<f32> {
    *self.get_projection_matrix() * self.matrix().inverse().unwrap()
  }

  fn get_view_matrix(&self) -> Mat4<f32> {
    self.matrix().inverse().unwrap()
  }

  fn get_vp_matrix_inverse(&self) -> Mat4<f32> {
    *self.matrix() * self.get_projection_matrix().inverse().unwrap()
  }
}

pub trait ResizableCamera: Camera {
  fn resize(&mut self, size: (f32, f32));
}
