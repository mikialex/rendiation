use crate::*;

#[derive(UniformBuffer)]
#[repr(align(16))]
pub struct MVPTransformation {
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
