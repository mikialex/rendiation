use bytemuck::*;
use rendiation_algebra::Vec3;
use rendiation_webgpu::ShaderUniformBlock;

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub _pad: f32,
  pub direction: Vec3<f32>,
  pub _pad2: f32,
}

impl ShaderUniformBlock for DirectionalLightShaderInfo {
  fn shader_struct() -> &'static str {
    "
      struct DirectionalLightShaderInfo {
        intensity: vec3<f32>;
        direction: vec3<f32>;
      };"
  }
}
