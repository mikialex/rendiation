use crate::*;

#[derive(UniformBuffer)]
#[repr(align(16))]
pub struct FogData {
  pub fog_color: Vec3<f32>,
  pub fog_end: f32,
  pub fog_start: f32,
}

glsl_function!(
  "
vec3 linear_fog(
  vec3 color, 
  vec3 fog_color, 
  float distance,
  float fog_start,
  float fog_end
){
  float effect = clamp((fog_end - distance) / (fog_end - fog_start), 0.0, 1.0);
  return mix(color, fog_color, 1.0 - effect);
}
"
);
