use rendiation_lighting_gpu_system::*;

use crate::*;

pub trait LightSystemSceneProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>>;
}

pub struct LightingComputeComponentGroupProvider {
  pub lights: Vec<Box<dyn LightSystemSceneProvider>>,
}

impl LightSystemSceneProvider for LightingComputeComponentGroupProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    Some(Box::new(LightingComputeComponentGroup {
      comps: self
        .lights
        .iter()
        .filter_map(|i| i.get_scene_lighting(scene))
        .collect(),
    }))
  }
}
