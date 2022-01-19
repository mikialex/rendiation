use bytemuck::*;
use rendiation_algebra::Vec3;
use rendiation_webgpu::ShaderUniformBlock;

use crate::{DirectShaderLight, ShaderLight};

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

impl ShaderLight for DirectionalLightShaderInfo {
  fn name() -> &'static str {
    "directional_light"
  }
}

impl DirectShaderLight for DirectionalLightShaderInfo {
  fn compute_direct_light() -> &'static str {
    "
      {
        directLight.color = directionalLight.color;
        directLight.direction = directionalLight.direction;
      }
    "
  }
}
