use crate::*;
use rendiation_algebra::Vec3;

pub type SceneLight<S> = SceneItemRef<SceneLightInner<S>>;

pub struct SceneLightInner<S: SceneContent> {
  pub light: S::Light,
  pub node: SceneNode,
}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub intensity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpotLight {
  pub intensity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
  pub intensity: Vec3<f32>,
}
