use crate::{math::*, Scene};

pub struct LightSampleResult {
  pub emissive: Vec3,
  pub light_in_dir: Vec3,
}

pub trait Light: Sync {
  fn sample(&self, world_position: Vec3, scene: &Scene) -> Option<LightSampleResult>;
}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub position: Vec3,
  pub color: Vec3,
}
