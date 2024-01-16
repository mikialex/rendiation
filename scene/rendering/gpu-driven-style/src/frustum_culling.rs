use crate::*;

pub struct ShaderFrustum {
  pub plane: [ShaderPlane; 6],
}

pub struct ShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

pub struct ShaderAABB {
  pub min: Vec3<f32>,
  pub max: Vec3<f32>,
}
