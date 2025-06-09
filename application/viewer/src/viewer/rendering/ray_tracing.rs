use rendiation_scene_rendering_gpu_ray_tracing::*;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingEffectMode {
  AO,
  ReferenceTracing,
}

pub struct RayTracingSystemGroup {
  pub base: RayTracingSystemBase,
  pub ao: RayTracingAORenderSystem,
  pub pt: DeviceReferencePathTracingSystem,
}

impl RayTracingSystemGroup {
  pub fn new(
    rtx: &RtxSystemCore,
    gpu: &GPU,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    Self {
      base: RayTracingSystemBase::new(rtx, gpu, camera_source),
      ao: RayTracingAORenderSystem::new(rtx, gpu),
      pt: DeviceReferencePathTracingSystem::new(rtx, gpu),
    }
  }
}

impl QueryBasedFeature<RayTracingFeatureGroup> for RayTracingSystemGroup {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, gpu: &GPU) {
    self.base.register(qcx, gpu);
    self.ao.register(qcx, gpu);
    self.pt.register(qcx, gpu);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    self.base.deregister(qcx);
    self.ao.deregister(qcx);
    self.pt.deregister(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> RayTracingFeatureGroup {
    RayTracingFeatureGroup {
      base: self.base.create_impl(cx),
      ao: self.ao.create_impl(cx),
      pt: self.pt.create_impl(cx),
    }
  }
}

pub struct RayTracingFeatureGroup {
  pub base: SceneRayTracingRendererBase,
  pub ao: SceneRayTracingAORenderer,
  pub pt: DeviceReferencePathTracingRenderer,
}
