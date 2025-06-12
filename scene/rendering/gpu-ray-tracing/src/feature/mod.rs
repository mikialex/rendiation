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

// // todo, share resource with the indirect renderer if possible
// pub struct RayTracingSystemBase {
//   camera: BoxedQueryBasedGPUFeature<Box<dyn RtxCameraRenderImpl>>,
//   scene_tlas: QueryToken,
//   mesh: MeshBindlessGPUSystemSource,
//   material: RtxSceneMaterialSource,
//   scene_ids: SceneIdProvider,
//   lighting: ScenePTLightingSource,
//   texture_system: TextureGPUSystemSource,
//   system: RtxSystemCore,
//   source_set: QueryCtxSetInfo,
// }

// impl RayTracingSystemBase {
//   pub fn new(
//     rtx: &RtxSystemCore,
//     gpu: &GPU,
//     camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
//   ) -> Self {
//     let tex_sys_ty = get_suitable_texture_system_ty(gpu, true, false);
//     Self {
//       scene_ids: Default::default(),
//       texture_system: TextureGPUSystemSource::new(tex_sys_ty),
//       camera: Box::new(DefaultRtxCameraRenderImplProvider::new(camera_source)),
//       scene_tlas: Default::default(),
//       system: rtx.clone(),
//       mesh: MeshBindlessGPUSystemSource::new(gpu),
//       lighting: ScenePTLightingSource::default(),
//       material: RtxSceneMaterialSource::default()
//         .with_material_support(PbrMRMaterialDefaultIndirectRenderImplProvider::default())
//         .with_material_support(PbrSGMaterialDefaultIndirectRenderImplProvider::default()),
//       source_set: Default::default(),
//     }
//   }
// }

pub struct SceneRayTracingRendererBase {
  pub camera: Box<dyn RtxCameraRenderImpl>,
  pub scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlASInstance>,
  pub mesh: MeshGPUBindlessImpl,
  pub material: SceneSurfaceSupport,
  pub lighting: ScenePTLightingSceneDataGroup,
  pub scene_ids: SceneIdUniformBufferAccess,
}

pub fn use_scene_rtx_renderer_base(
  cx: &mut impl QueryGPUHookCx,
  system: &RtxSystemCore,
  camera: Option<Box<dyn RtxCameraRenderImpl>>,
  mesh: Option<MeshGPUBindlessImpl>,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  tex: Option<GPUTextureBindingSystem>,
) -> Option<SceneRayTracingRendererBase> {
  let material = use_rtx_scene_material(cx, materials, tex);

  let scene_tlas = cx.use_reactive_query_gpu(|gpu| scene_to_tlas(gpu, system.rtx_acc.clone()));

  let lighting = use_scene_pt_light_source(cx);
  let scene_ids = use_scene_id_provider(cx); // this could be reused, but it's unnecessary.

  cx.when_render(|| SceneRayTracingRendererBase {
    camera: camera.unwrap(),
    scene_tlas: scene_tlas.unwrap(),
    mesh: mesh.unwrap(),
    material: material.unwrap(),
    lighting: lighting.unwrap(),
    scene_ids: scene_ids.unwrap(),
  })
}
