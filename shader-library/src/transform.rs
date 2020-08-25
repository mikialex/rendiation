use crate::*;

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct MVPTransformation {
  pub mvp: Mat4<f32>,
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
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
