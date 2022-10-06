use crate::*;
use rendiation_algebra::Vec3;

pub type SceneLight<T> = SceneItemRef<SceneLightInner<T>>;

pub struct SceneLightInner<T> {
  pub light: T,
  /// Note: Light properties are unaffected by node transforms
  /// — for example, range and intensity do not change with scale.
  pub node: SceneNode,
}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub intensity: Vec3<f32>,
  pub cutoff_distance: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SpotLight {
  pub intensity: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_angle: f32,
  /// should less equal to half_cont_angle,large equal to zero
  pub half_penumbra_angle: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
  pub intensity: Vec3<f32>,
}
