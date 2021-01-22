use crate::*;

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct CameraTransform {
  pub projection: Mat4<f32>,
}

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct ObjectTransform {
  model_view_matrix: Mat4<f32>,
  normal_matrix: Mat3<f32>,
}

impl Default for ObjectTransform {
  fn default() -> Self {
    Self {
      model_view_matrix: Mat4::one(),
      normal_matrix: Mat3::one(),
    }
  }
}

impl Default for CameraTransform {
  fn default() -> Self {
    Self {
      projection: Mat4::one(),
    }
  }
}

impl CameraTransform {
  pub fn apply(
    projection: Node<Mat4<f32>>,
    model_view: Node<Mat4<f32>>,
    raw_position: Node<Vec3<f32>>,
    builder: &ShaderGraphBuilder,
  ) -> (Node<Vec4<f32>>, Node<Vec4<f32>>) {
    let mv_position = to_mv_position(raw_position, model_view);
    let clip_position = apply_projection(mv_position, projection);
    builder.set_vertex_root(clip_position);
    (clip_position, mv_position)
  }
}

glsl_function!(
  "
vec4 mvp_projection(vec3 raw, mat4 projection, mat4 model_view){
    return projection * model_view * vec4(raw, 1.0);
}
"
);

glsl_function!(
  "
vec4 apply_projection(vec4 mv_position, mat4 projection){
    return projection * (-mv_position);
}
"
);

glsl_function!(
  "
vec4 to_mv_position(vec3 raw, mat4 model_view){
    return -(model_view * vec4(raw, 1.0));
}
"
);
