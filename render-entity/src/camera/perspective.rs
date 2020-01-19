
use crate::transformed_object::TransformedObject;
use super::Camera;
use rendiation_math_entity::*;
use rendiation_math::*;

#[derive(Default)]
pub struct PerspectiveCamera {
  pub projection_matrix: Mat4<f32>,
  pub transform: Transformation,

  pub near: f32,
  pub far: f32,
  pub fov: f32,
  pub aspect: f32,
  pub zoom: f32,
}

impl PerspectiveCamera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::<f32>::one(),
      transform: Transformation::new(),
      near: 1.,
      far: 100_000.,
      fov: 45.,
      aspect: 1.,
      zoom: 1.,
    }
  }
}

impl TransformedObject for PerspectiveCamera {
  fn get_transform(&self) -> &Transformation{
    &self.transform
  }
}

impl Camera for PerspectiveCamera {
  fn update_projection(&mut self) {
    let top = self.near * (f32::pi_by_c180() * 0.5 * self.fov).tan() / self.zoom;
    let height = 2. * top;
    let width = self.aspect * height;
    let left = -0.5 * width;
    self.projection_matrix.make_perspective(
      left,
      left + width,
      top,
      top - height,
      self.near,
      self.far,
    );
  }

  fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
    self.update_projection();
  }
}