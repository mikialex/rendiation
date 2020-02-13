use super::Camera;
use crate::transformed_object::TransformedObject;
use rendiation_math::*;
use rendiation_math_entity::*;

pub struct OrthographicCamera {
  pub left: f32,
  pub right: f32,
  pub top: f32,
  pub bottom: f32,
  pub near: f32,
  pub far: f32,
  transform: Transformation,
  projection_matrix: Mat4<f32>,
}

impl OrthographicCamera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::<f32>::one(),
      transform: Transformation::new(),
      left: -1.0,
      right: 1.0,
      top: 1.0,
      bottom: -1.0,
      near: 0.01,
      far: 1000.0,
    }
  }
}

impl TransformedObject for OrthographicCamera {
  fn get_transform(&self) -> &Transformation {
    &self.transform
  }

  fn get_transform_mut(&mut self) -> &mut Transformation {
    &mut self.transform
  }
}

impl Camera for OrthographicCamera {
  fn update_projection(&mut self) {
    self.projection_matrix = Mat4::ortho_rh(
      self.left,
      self.right,
      self.bottom,
      self.top,
      self.near,
      self.far,
    );
  }

  fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  fn resize(&mut self, size: (f32, f32)) {
    // self.aspect = size.0 / size.1;
    self.update_projection();
  }
}
