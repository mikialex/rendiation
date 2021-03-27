use crate::*;

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct FogData {
  pub fog_color: Vec4<f32>,
  pub fog_end: f32,
  pub fog_start: f32,
}

impl Default for FogData {
  fn default() -> Self {
    Self {
      fog_color: Vec4::new(1., 1., 1., 1.),
      fog_end: 0.,
      fog_start: 100.,
    }
  }
}

impl FogData {
  pub fn apply_fog(
    fog: <FogData as ShaderGraphBindGroupItemProvider>::ShaderGraphBindGroupItemInstance,
    input: Node<Vec3<f32>>,
    distance: Node<f32>,
  ) -> Node<Vec3<f32>> {
    linear_fog(input, fog.fog_color, distance, fog.fog_start, fog.fog_end)
  }
}

glsl_function!(
  "
vec3 linear_fog(
  vec3 color, 
  vec4 fog_color, 
  float distance,
  float fog_start,
  float fog_end
){
  float effect = clamp((fog_end - distance) / (fog_end - fog_start), 0.0, 1.0);
  return mix(color, fog_color.xyz, 1.0 - effect);
}
"
);
