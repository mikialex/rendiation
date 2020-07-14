use super::Camera;
use crate::{raycaster::Raycaster, transformed_object::TransformedObject, ResizableCamera};
use rendiation_math::*;
use rendiation_math_entity::*;

pub struct OrthographicCamera {
  pub left: f32,
  pub right: f32,
  pub top: f32,
  pub bottom: f32,
  pub near: f32,
  pub far: f32,
  world_matrix: Mat4<f32>,
  projection_matrix: Mat4<f32>,
}

impl OrthographicCamera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::one(),
      world_matrix: Mat4::one(),
      left: -50.0,
      right: 50.0,
      top: 50.0,
      bottom: -50.0,
      near: 0.0,
      far: 1000.0,
    }
  }
}

impl Raycaster for OrthographicCamera {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3 {
    let coords_x = view_position.x * 2. - 1.;
    let coords_y = view_position.y * 2. - 1.;

    let origin = Vec3::new(
      coords_x,
      coords_y,
      (self.near + self.far) / (self.near - self.far),
    ) * self.get_vp_matrix_inverse();
    let direction = Vec3::new(0., 0., -1.).transform_direction(*self.matrix());
    Ray3::new(origin, direction)
  }
}

impl TransformedObject for OrthographicCamera {
  fn matrix(&self) -> &Mat4<f32> {
    &self.world_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.world_matrix
  }
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl Camera for OrthographicCamera {
  fn update_projection(&mut self) {
    self.projection_matrix = Mat4::ortho(
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
}

pub struct ViewFrustumOrthographicCamera {
  camera: OrthographicCamera,
  aspect: f32,
  frustum_size: f32,
}

impl ViewFrustumOrthographicCamera {
  pub fn new() -> Self {
    ViewFrustumOrthographicCamera {
      camera: OrthographicCamera::new(),
      aspect: 1.,
      frustum_size: 50.,
    }
  }
}

impl Raycaster for ViewFrustumOrthographicCamera {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3 {
    self.camera.create_screen_ray(view_position)
  }
}

impl TransformedObject for ViewFrustumOrthographicCamera {
  fn matrix(&self) -> &Mat4<f32> {
    &self.camera.world_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.camera.world_matrix
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl Camera for ViewFrustumOrthographicCamera {
  fn update_projection(&mut self) {
    self.camera.left = self.frustum_size * self.aspect / -2.;
    self.camera.right = self.frustum_size * self.aspect / 2.;
    self.camera.top = self.frustum_size / 2.;
    self.camera.bottom = self.frustum_size / -2.;

    self.camera.update_projection();
  }

  fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.camera.projection_matrix
  }
}

impl ResizableCamera for ViewFrustumOrthographicCamera {
  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
    println!("{}", self.aspect);
    self.update_projection();
  }
}
