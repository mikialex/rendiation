use rendiation_algebra::*;
use rendiation_geometry::Ray3;

pub struct Camera {
  pub projection_matrix: Mat4<f32>,
  pub matrix: Mat4<f32>,
}

impl Default for Camera {
  fn default() -> Self {
    Self::new()
  }
}

impl Camera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::one(),
      matrix: Mat4::one(),
    }
  }

  pub fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  pub fn get_vp_matrix(&self) -> Mat4<f32> {
    self.projection_matrix * self.matrix.inverse_or_identity()
  }

  pub fn get_view_matrix(&self) -> Mat4<f32> {
    self.matrix.inverse_or_identity()
  }

  pub fn get_vp_matrix_inverse(&self) -> Mat4<f32> {
    self.matrix * self.projection_matrix.inverse_or_identity()
  }

  pub fn create_screen_ray(&self, screen_position: Vec2<f32>) -> Ray3 {
    let origin = self.matrix.position();
    let target = self.get_vp_matrix_inverse()
      * Vec3::new(
        screen_position.x * 2. - 1.,
        screen_position.y * 2. - 1.,
        0.5,
      );
    let direction = (target - origin).into_normalized();
    Ray3::new(origin, direction)
  }
}
