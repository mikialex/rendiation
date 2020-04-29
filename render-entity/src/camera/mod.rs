use crate::transformed_object::TransformedObject;
use rendiation_math::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

/// Camera is a combine of projection matrix and transformation
///
/// Different camera has different internal states and
/// projection update methods
pub trait Camera: TransformedObject {
  fn update_projection(&mut self);
  fn get_projection_matrix(&self) -> &Mat4<f32>;

  fn get_vp_matrix(&self) -> Mat4<f32> {
    *self.get_projection_matrix() * self.get_transform().matrix.inverse().unwrap()
  }

  fn get_vp_matrix_inverse(&self) -> Mat4<f32> {
    self.get_transform().matrix * self.get_projection_matrix().inverse().unwrap()
  }
}

/// ResizeAble Camera is a camera that can response to canvas size change
pub trait ResizableCamera: Camera {
  fn resize(&mut self, size: (f32, f32));
}
