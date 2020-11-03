use crate::*;

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct CameraTransform {
  pub mvp: Mat4<f32>,
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
}

impl Default for CameraTransform {
  fn default() -> Self {
    Self {
      mvp: Mat4::one(),
      projection: Mat4::one(),
      model_view: Mat4::one(),
    }
  }
}

impl CameraTransform {
  pub fn apply(
    transform: <Self as ShaderGraphBindGroupItemProvider>::ShaderGraphBindGroupItemInstance,
    raw_position: ShaderGraphNodeHandle<Vec3<f32>>,
    builder: &ShaderGraphBuilder,
  ) -> (
    ShaderGraphNodeHandle<Vec4<f32>>,
    ShaderGraphNodeHandle<Vec4<f32>>,
  ) {
    let mv_position = to_mv_position(raw_position, transform.model_view);
    let clip_position = apply_projection(mv_position, transform.projection);
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
