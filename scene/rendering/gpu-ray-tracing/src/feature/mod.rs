mod path_tracing;
pub use path_tracing::*;

mod ao;
pub use ao::*;

use crate::*;

// todo support max mesh count grow
pub const MAX_MODEL_COUNT_IN_SBT: u32 = 2048;

#[derive(Clone)]
pub struct RtxSystemCore {
  pub rtx_system: Box<dyn GPURaytracingSystem>,
  pub rtx_device: Box<dyn GPURayTracingDeviceProvider>,
  pub rtx_acc: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl RtxSystemCore {
  pub fn new(rtx: Box<dyn GPURaytracingSystem>) -> Self {
    Self {
      rtx_device: rtx.create_raytracing_device(),
      rtx_acc: rtx.create_acceleration_structure_system(),
      rtx_system: rtx,
    }
  }
}

pub struct RayTracingSystemBase {
  camera: Box<dyn RenderImplProvider<Box<dyn RtxCameraRenderImpl>>>,
  scene_tlas: UpdateResultToken,
  mesh: MeshBindlessGPUSystemSource,
  system: RtxSystemCore,
}

impl RayTracingSystemBase {
  pub fn new(
    rtx: &RtxSystemCore,
    gpu: &GPU,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    Self {
      camera: Box::new(DefaultRtxCameraRenderImplProvider::new(camera_source)),
      scene_tlas: Default::default(),
      system: rtx.clone(),
      mesh: MeshBindlessGPUSystemSource::new(gpu),
    }
  }
}

pub struct SceneRayTracingRendererBase {
  pub camera: Box<dyn RtxCameraRenderImpl>,
  pub rtx_system: Box<dyn GPURaytracingSystem>,
  pub scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlASInstance>,
  pub mesh: MeshGPUBindlessImpl,
}

impl RenderImplProvider<SceneRayTracingRendererBase> for RayTracingSystemBase {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.scene_tlas =
      source.register_reactive_query(scene_to_tlas(cx, self.system.rtx_acc.clone()));
    self.camera.register_resource(source, cx);
    self.mesh.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.scene_tlas);
    self.camera.deregister_resource(source);
    self.mesh.deregister_resource(source);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> SceneRayTracingRendererBase {
    SceneRayTracingRendererBase {
      scene_tlas: res.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(res),
      rtx_system: self.system.rtx_system.clone(),
      mesh: self.mesh.create_impl_internal_impl(res),
    }
  }
}
