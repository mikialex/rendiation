use crate::*;
#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct ShaderFrustum {
  // pub plane: [ShaderPlane; 6],
  pub plane_0: ShaderPlane,
  pub plane_1: ShaderPlane,
  pub plane_2: ShaderPlane,
  pub plane_3: ShaderPlane,
  pub plane_4: ShaderPlane,
  pub plane_5: ShaderPlane,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct ShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct ShaderAABB {
  pub min: Vec3<f32>,
  pub max: Vec3<f32>,
}
