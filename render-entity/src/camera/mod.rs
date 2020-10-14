use crate::{transformed_object::TransformedObject, Raycaster};
use rendiation_math::*;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;
use rendiation_math_entity::Ray3;

pub struct Camera {
  projection_matrix: Mat4<f32>,
  local_matrix: Mat4<f32>,
}

impl Camera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::one(),
      local_matrix: Mat4::one(),
    }
  }

  pub fn update_by(&mut self, projection: &impl Projection) {
    projection.update_projection(&mut self.projection_matrix);
  }

  pub fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  pub fn get_vp_matrix(&self) -> Mat4<f32> {
    self.projection_matrix * self.local_matrix.inverse().unwrap()
  }

  pub fn get_view_matrix(&self) -> Mat4<f32> {
    self.local_matrix.inverse().unwrap()
  }

  pub fn get_vp_matrix_inverse(&self) -> Mat4<f32> {
    self.local_matrix * self.projection_matrix.inverse().unwrap()
  }
}

pub trait Projection {
  fn update_projection(&self, projection: &mut Mat4<f32>);
  fn update(&self, camera: &mut Camera) {
    self.update_projection(&mut camera.projection_matrix);
  }
}

pub trait ResizableProjection: Projection {
  fn resize(&mut self, size: (f32, f32));
}

impl Raycaster for Camera {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3 {
    let origin = self.matrix().position();
    let target = Vec3::new(view_position.x * 2. - 1., view_position.y * 2. - 1., 0.5)
      * self.get_vp_matrix_inverse();
    let direction = (target - origin).normalize();
    Ray3::new(origin, direction)
  }
}

impl TransformedObject for Camera {
  fn matrix(&self) -> &Mat4<f32> {
    &self.local_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.local_matrix
  }
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}
