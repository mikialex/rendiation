use super::Camera;
use crate::raycaster::Raycaster;
use crate::{transformed_object::TransformedObject, ResizableCamera};
use rendiation_math::*;
use rendiation_math_entity::*;

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
      fov: 90.,
      aspect: 1.,
      zoom: 1.,
    }
  }
}

impl Raycaster for PerspectiveCamera {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray {
    let origin = self.get_transform().matrix.position();
    let target = Vec3::new(view_position.x * 2. - 1., view_position.y * 2. - 1., 0.5)
      * self.get_vp_matrix_inverse();
    let direction = (target - origin).normalize();
    Ray::new(origin, direction)
  }
}

impl TransformedObject for PerspectiveCamera {
  fn get_transform(&self) -> &Transformation {
    &self.transform
  }

  fn get_transform_mut(&mut self) -> &mut Transformation {
    &mut self.transform
  }
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl Camera for PerspectiveCamera {
  fn update_projection(&mut self) {
    self.projection_matrix = Mat4::perspective_fov_rh(self.fov, self.aspect, self.near, self.far);
  }

  fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }
}

impl ResizableCamera for PerspectiveCamera {
  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
    self.update_projection();
  }
}
