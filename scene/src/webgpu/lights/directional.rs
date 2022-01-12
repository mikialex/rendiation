use bytemuck::*;
use rendiation_algebra::Vec3;

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub _pad: f32,
  pub direction: Vec3<f32>,
  pub _pad2: f32,
}
