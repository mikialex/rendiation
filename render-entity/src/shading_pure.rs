pub struct PureColorShading {
  pub index: usize,
  pub vertex: String,
  pub frag: String,
  // // uniforms
  // projection_matrix: Mat4<f32>,
  // world_matrix: Mat4<f32>,
  // camera_inverse_matrix: Mat4<f32>,
}

impl PureColorShading {
  pub fn new(index: usize) -> Self {
    PureColorShading {
        index,
        vertex: String::from(
          r#"
          attribute vec3 position;
              uniform mat4 model_matrix;
              uniform mat4 camera_inverse;
              uniform mat4 projection_matrix;
              void main() {
                gl_Position = projection_matrix * camera_inverse * model_matrix * vec4(position, 1.0);
              }
          "#,
        ),
        frag: String::from(
          r#"
              void main() {
                  gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
              }
          "#,
        ),
        // projection_matrix: Mat4::one(),
        // world_matrix: Mat4::one(),
        // camera_inverse_matrix:Mat4::one(),
      }
  }}
