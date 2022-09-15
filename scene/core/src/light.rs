use crate::*;
use rendiation_algebra::Vec3;

pub type SceneLight<S> = SceneItemRef<<S as SceneContent>::Light>;

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
