use rendiation_lighting_gpu_system::*;

use crate::*;

#[derive(Default)]
pub struct DifferentLightRenderImplProvider {
  lights: Vec<Box<dyn RenderImplProvider<Box<dyn LightingComputeComponent>>>>,
}

impl DifferentLightRenderImplProvider {
  pub fn with_light(
    mut self,
    impls: impl RenderImplProvider<Box<dyn LightingComputeComponent>> + 'static,
  ) -> Self {
    self.lights.push(Box::new(impls));
    self
  }
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for DifferentLightRenderImplProvider {
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
  ) -> Box<dyn LightingComputeComponent> {
    Box::new(LightingComputeComponentGroup {
      comps: self.lights.iter().map(|i| i.create_impl(res)).collect(),
    })
  }
}
