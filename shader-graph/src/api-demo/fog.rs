#[UniformBuffer]
struct FogData {
  density: f32,
  fog_color: Vec3<f32>,
  fog_end: f32,
  fog_start: f32,
}

impl Fog {
  pub fn create_fog(input_color: ShaderGraphNodeHandle<Vec3>) -> ShaderGraphNodeHandle<Vec3> {
    todo!()
  }
}

glsl!(
  "
vec3 linear_fog(vec3 color, float distance){
  return clamp((fog_end - distance) / (fog_end - fog_start), 0.0, 1.0);
}
"
);
