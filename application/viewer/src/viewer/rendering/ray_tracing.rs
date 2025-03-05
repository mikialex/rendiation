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
      ao: RayTracingAORenderSystem::new(rtx),
      pt: DeviceReferencePathTracingSystem::new(rtx),
    }
  }
}

impl RenderImplProvider<RayTracingFeatureGroup> for RayTracingSystemGroup {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.base.register_resource(source, cx);
    self.ao.register_resource(source, cx);
    self.pt.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.base.deregister_resource(source);
    self.ao.deregister_resource(source);
    self.pt.deregister_resource(source);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> RayTracingFeatureGroup {
    RayTracingFeatureGroup {
      base: self.base.create_impl(res),
      ao: self.ao.create_impl(res),
      pt: self.pt.create_impl(res),
    }
  }
}

pub struct RayTracingFeatureGroup {
  pub base: SceneRayTracingRendererBase,
  pub ao: SceneRayTracingAORenderer,
  pub pt: DeviceReferencePathTracingRenderer,
}
