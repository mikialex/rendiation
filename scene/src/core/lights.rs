use rendiation_algebra::Vec3;

use crate::SceneNodeHandle;

pub struct SceneLight {
  pub light: Box<dyn Light>,
  pub node: SceneNodeHandle,
}

pub trait Light {}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub intensity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpotLight {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
}

#[repr(C)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub _pad: f32,
  pub direction: Vec3<f32>,
  pub _pad2: f32,
}
