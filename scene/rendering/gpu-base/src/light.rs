use rendiation_lighting_gpu_system::*;

use crate::*;

pub trait LightSystemSceneProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn LightingComputeComponent>;
}

#[derive(Default)]
pub struct DifferentLightRenderImplProvider {
  lights: Vec<Box<dyn RenderImplProvider<Box<dyn LightSystemSceneProvider>>>>,
}

impl DifferentLightRenderImplProvider {
  pub fn with_light(
    mut self,
    impls: impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> + 'static,
  ) -> Self {
    self.lights.push(Box::new(impls));
    self
  }
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for DifferentLightRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self
      .lights
      .iter_mut()
      .for_each(|l| l.register_resource(source, cx));
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self
      .lights
      .iter_mut()
      .for_each(|l| l.deregister_resource(source));
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn LightSystemSceneProvider> {
    Box::new(LightingComputeComponentGroupProvider {
      lights: self.lights.iter().map(|i| i.create_impl(res)).collect(),
    })
  }
}

struct LightingComputeComponentGroupProvider {
  lights: Vec<Box<dyn LightSystemSceneProvider>>,
}

impl LightSystemSceneProvider for LightingComputeComponentGroupProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn LightingComputeComponent> {
    Box::new(LightingComputeComponentGroup {
      comps: self
        .lights
        .iter()
        .map(|i| i.get_scene_lighting(scene))
        .collect(),
    })
  }
}
