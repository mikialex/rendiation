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
  scene_ids: SceneIdProvider,
  lighting: ScenePTLightingSource,
  texture_system: TextureGPUSystemSource,
  system: RtxSystemCore,
  source_set: QueryCtxSetInfo,
}

impl RayTracingSystemBase {
  pub fn new(
    rtx: &RtxSystemCore,
    gpu: &GPU,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    let tex_sys_ty = get_suitable_texture_system_ty(gpu, true, false);
    Self {
      scene_ids: Default::default(),
      texture_system: TextureGPUSystemSource::new(tex_sys_ty),
      camera: Box::new(DefaultRtxCameraRenderImplProvider::new(camera_source)),
      scene_tlas: Default::default(),
      system: rtx.clone(),
      mesh: MeshBindlessGPUSystemSource::new(gpu),
      lighting: ScenePTLightingSource::default(),
      material: RtxSceneMaterialSource::default()
        .with_material_support(PbrMRMaterialDefaultIndirectRenderImplProvider::default())
        .with_material_support(PbrSGMaterialDefaultIndirectRenderImplProvider::default()),
      source_set: Default::default(),
    }
  }
}

pub struct SceneRayTracingRendererBase {
  pub camera: Box<dyn RtxCameraRenderImpl>,
  pub rtx_system: Box<dyn GPURaytracingSystem>,
  pub scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlASInstance>,
  pub mesh: MeshGPUBindlessImpl,
  pub material: SceneSurfaceSupport,
  pub lighting: ScenePTLightingSceneDataGroup,
  pub scene_ids: SceneIdUniformBufferAccess,
  pub any_changed: bool,
}

impl QueryBasedFeature<SceneRayTracingRendererBase> for RayTracingSystemBase {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    qcx.record_new_registered(&mut self.source_set);
    self.scene_tlas = qcx.register_reactive_query(scene_to_tlas(cx, self.system.rtx_acc.clone()));
    self.camera.register(qcx, cx);
    self.mesh.register(qcx, cx);
    self.material.register_resource(qcx, cx);
    self.texture_system.register_resource(qcx, cx);
    self.lighting.register_resource(qcx, cx);
    self.scene_ids.register(qcx, cx);
    qcx.end_record(&mut self.source_set);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.scene_tlas);
    self.camera.deregister(qcx);
    self.mesh.deregister(qcx);
    self.material.deregister_resource(qcx);
    self.texture_system.deregister_resource(qcx);
    self.lighting.deregister_resource(qcx);
    self.scene_ids.deregister(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> SceneRayTracingRendererBase {
    let tex = self.texture_system.create_impl(cx);
    let any_changed = cx.has_any_changed_in_set(&self.source_set);
    SceneRayTracingRendererBase {
      scene_tlas: cx.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(cx),
      rtx_system: self.system.rtx_system.clone(),
      mesh: self.mesh.create_impl_internal_impl(cx),
      material: self.material.create_impl(cx, &tex),
      lighting: self.lighting.create_impl(cx),
      scene_ids: self.scene_ids.create_impl(cx),
      any_changed,
    }
  }
}
