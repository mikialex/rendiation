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

// todo, share resource with the indirect renderer if possible
pub struct RayTracingSystemBase {
  camera: BoxedQueryBasedGPUFeature<Box<dyn RtxCameraRenderImpl>>,
  scene_tlas: QueryToken,
  mesh: MeshBindlessGPUSystemSource,
  material: RtxSceneMaterialSource,
  lighting: ScenePTLightingSource,
  texture_system: TextureGPUSystemSource,
  system: RtxSystemCore,
}

impl RayTracingSystemBase {
  pub fn new(
    rtx: &RtxSystemCore,
    gpu: &GPU,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    let tex_sys_ty = get_suitable_texture_system_ty(gpu, true, false);
    Self {
      texture_system: TextureGPUSystemSource::new(tex_sys_ty),
      camera: Box::new(DefaultRtxCameraRenderImplProvider::new(camera_source)),
      scene_tlas: Default::default(),
      system: rtx.clone(),
      mesh: MeshBindlessGPUSystemSource::new(gpu),
      lighting: ScenePTLightingSource::default(),
      material: RtxSceneMaterialSource::default()
        .with_material_support(PbrMRMaterialDefaultIndirectRenderImplProvider::default())
        .with_material_support(PbrSGMaterialDefaultIndirectRenderImplProvider::default()),
    }
  }
}

pub struct SceneRayTracingRendererBase {
  pub camera: Box<dyn RtxCameraRenderImpl>,
  pub rtx_system: Box<dyn GPURaytracingSystem>,
  pub scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlASInstance>,
  pub mesh: MeshGPUBindlessImpl,
  pub material: SceneSurfaceSupport,
  pub lighting: ScenePTLighting,
}

impl QueryBasedFeature<SceneRayTracingRendererBase> for RayTracingSystemBase {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.scene_tlas = qcx.register_reactive_query(scene_to_tlas(cx, self.system.rtx_acc.clone()));
    self.camera.register(qcx, cx);
    self.mesh.register(qcx, cx);
    self.material.register_resource(qcx, cx);
    self.texture_system.register_resource(qcx, cx);
    self.lighting.register_resource(qcx, cx);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.scene_tlas);
    self.camera.deregister(qcx);
    self.mesh.deregister(qcx);
    self.material.deregister_resource(qcx);
    self.texture_system.deregister_resource(qcx);
    self.lighting.deregister_resource(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> SceneRayTracingRendererBase {
    let tex = self.texture_system.create_impl(cx);
    SceneRayTracingRendererBase {
      scene_tlas: cx.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(cx),
      rtx_system: self.system.rtx_system.clone(),
      mesh: self.mesh.create_impl_internal_impl(cx),
      material: self.material.create_impl(cx, &tex),
      lighting: self.lighting.create_impl(cx),
    }
  }
}
