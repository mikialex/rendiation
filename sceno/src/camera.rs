use rendiation_algebra::*;

pub struct Camera {
  pub projection_matrix: Mat4<f32>,
  pub matrix: Mat4<f32>,
  pub matrix_inverse: Mat4<f32>,
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
      matrix_inverse: Mat4::one(),
    }
  }

  pub fn update_by(&mut self, projection: &impl Projection) {
    projection.update_projection(&mut self.projection_matrix);
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
}
