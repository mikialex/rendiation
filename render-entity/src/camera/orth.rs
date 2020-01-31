use crate::transformed_object::TransformedObject;
use super::Camera;
use rendiation_math_entity::*;
use rendiation_math::*;

pub struct OrthographicCamera {
  pub left: f32,
  pub right: f32,
  pub top: f32,
  pub bottom: f32,
  pub near: f32,
  pub far: f32,
  transform: Transformation,
  projection_matrix: Mat4<f32>
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