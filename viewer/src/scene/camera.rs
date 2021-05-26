use rendiation_algebra::Mat4;

pub struct Camera {
  pub projection_matrix: Mat4<f32>,
  pub view_matrix: Mat4<f32>,
  pub matrix: Mat4<f32>,
}
