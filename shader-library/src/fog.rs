use crate::*;

#[derive(UniformBuffer)]
#[repr(align(16))]
struct FogData {
  pub fog_color: Vec4<f32>,
  pub fog_end: f32,
  pub fog_start: f32,
}

glsl_function!(
  "
vec4 linear_fog(vec4 color, float distance){
  float effect = clamp((fog_end - distance) / (fog_end - fog_start), 0.0, 1.0);
  return mix(color, fog_color, 1.0 - effect);
}
"
);
