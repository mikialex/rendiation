mod perspective;
pub use perspective::*;
use rendiation_math::*;

/// Camera is a combine of projection matrix and transformation
/// 
/// Different camera has different internal states and 
/// projection update methods
pub trait Camera {
  fn update_projection(&mut self);
  fn get_projection_matrix(&self) -> &Mat4<f32>;
  fn get_world_matrix(&self) -> &Mat4<f32>;
  fn resize(&mut self, size: (f32, f32));

  fn get_vp_matrix(&self) -> Mat4<f32> {
    *self.get_projection_matrix() * (*self.get_world_matrix())
  }
}
