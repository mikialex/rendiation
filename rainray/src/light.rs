use crate::{math::*, Scene};
use rendiation_algebra::{InnerProductSpace, IntoNormalizedVector};

pub struct LightSampleResult {
  pub emissive: Vec3,
  pub light_in_dir: NormalizedVec3,
}

pub trait Light: Sync + 'static {
  fn sample(&self, world_position: Vec3, scene: &Scene) -> Option<LightSampleResult>;
}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub position: Vec3,
  pub intensity: Vec3,
}

impl Light for PointLight {
  fn sample(&self, world_position: Vec3, scene: &Scene) -> Option<LightSampleResult> {
    if !scene.test_point_visible_to_point(self.position, world_position) {
      return None;
    }
    let light_in_dir = world_position - self.position;
    let distance = light_in_dir.length();
    Some(LightSampleResult {
      emissive: self.intensity / (distance * distance),
      light_in_dir: light_in_dir.into_normalized(),
    })
  }
}
