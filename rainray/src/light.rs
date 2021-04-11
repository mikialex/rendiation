use crate::{math::*, RainraySceneExt, Scene, SceneNode};
use rendiation_algebra::{InnerProductSpace, IntoNormalizedVector};
use sceno::PointLight;

pub struct LightSampleResult {
  pub emissive: Vec3,
  pub light_in_dir: NormalizedVec3,
}

pub trait Light: Sync + 'static {
  fn sample(
    &self,
    world_position: Vec3,
    scene: &Scene,
    node: &SceneNode,
  ) -> Option<LightSampleResult>;
}

pub trait LightToBoxed: Light + Sized {
  fn to_boxed(self) -> Box<dyn Light> {
    Box::new(self) as Box<dyn Light>
  }
}

impl LightToBoxed for PointLight {}
impl Light for PointLight {
  fn sample(
    &self,
    world_position: Vec3,
    scene: &Scene,
    node: &SceneNode,
  ) -> Option<LightSampleResult> {
    let light_position = node.world_matrix.position();

    if !scene.test_point_visible_to_point(light_position, world_position) {
      return None;
    }
    let light_in_dir = world_position - light_position;
    let distance = light_in_dir.length();
    Some(LightSampleResult {
      emissive: self.intensity / (distance * distance),
      light_in_dir: light_in_dir.into_normalized(),
    })
  }
}
