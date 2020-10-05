use crate::*;

// struct A {
//   t: <Vec3<f32> as rendiation_webgl::WebGLUniformUploadable>::UploadInstance,
// }

// use rendiation_webgl::UploadInstance;
// impl A {
//   fn te(
//     &self,
//     gl: &rendiation_webgl::WebGl2RenderingContext,
//     r: &rendiation_ral::ResourceManager<rendiation_webgl::WebGLRenderer>,
//   ) {
//     // (self.t as UploadInstance<Vec3<f32>>).upload(&Vec3::new(0.0, 0.0, 0.0), gl, r);
//     <Vec3<f32> as rendiation_webgl::WebGLUniformUploadable>::upload(
//       &Vec3::new(0.0, 0.0, 0.0),
//       &mut self.t,
//       gl,
//       r,
//     );
//   }
// }

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct FogData {
  pub fog_color: Vec4<f32>,
  pub fog_end: f32,
  pub fog_start: f32,
}

impl FogData {
  pub fn apply_fog(
    fog: <FogData as ShaderGraphBindGroupItemProvider>::ShaderGraphBindGroupItemInstance,
    input: ShaderGraphNodeHandle<Vec3<f32>>,
    distance: ShaderGraphNodeHandle<f32>,
  ) -> ShaderGraphNodeHandle<Vec3<f32>> {
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
