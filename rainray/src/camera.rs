use rendiation_algebra::*;
use rendiation_geometry::Ray3;

pub struct Camera {
  pub projection_matrix: Mat4<f32>,
  pub matrix: Mat4<f32>,
  pub matrix_inverse: Mat4<f32>,
}

impl Camera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::one(),
      matrix: Mat4::one(),
      matrix_inverse: Mat4::one(),
    }
  }

  pub fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  pub fn get_vp_matrix(&self) -> Mat4<f32> {
    self.projection_matrix * self.matrix.inverse().unwrap()
  }

  pub fn get_view_matrix(&self) -> Mat4<f32> {
    self.matrix.inverse().unwrap()
  }

  pub fn get_vp_matrix_inverse(&self) -> Mat4<f32> {
    self.matrix * self.projection_matrix.inverse().unwrap()
  }

  pub fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3 {
    let origin = self.matrix.position();
    let target = Vec3::new(view_position.x * 2. - 1., view_position.y * 2. - 1., 0.5)
      * self.get_vp_matrix_inverse();
    let direction = (target - origin).into_normalized();
    Ray3::new(origin, direction)
  }
}
