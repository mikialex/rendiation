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

pub struct SceneRayTracingRendererBase {
  pub camera: Box<dyn RtxCameraRenderImpl>,
  pub scene_tlas: BoxedDynQuery<RawEntityHandle, TlASInstance>,
  pub mesh: MeshGPUBindlessImpl,
  pub material: SceneSurfaceSupport,
  pub lighting: ScenePTLightingSceneDataGroup,
  pub scene_ids: SceneIdUniformBufferAccess,
}

pub fn use_scene_rtx_renderer_base(
  cx: &mut QueryGPUHookCx,
  system: &RtxSystemCore,
  camera: Option<Box<dyn RtxCameraRenderImpl>>,
  mesh: Option<MeshGPUBindlessImpl>,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  tex: Option<GPUTextureBindingSystem>,
) -> Option<(SceneRayTracingRendererBase, bool)> {
  let (cx, scope) = cx.use_begin_change_set_collect();

  let material = use_rtx_scene_material(cx, materials, tex);

  let scene_tlas = use_scene_to_tlas(cx, &system.rtx_acc);

  let lighting = use_scene_pt_light_source(cx);
  let scene_ids = use_scene_id_provider(cx); // this could be reused, but it's unnecessary.

  let changed = scope(cx);
  let (cx, changed_s) = cx.use_plain_state(|| false);
  if let Some(changed) = changed {
    *changed_s |= changed;
  }

  cx.when_render(|| {
    (
      SceneRayTracingRendererBase {
        camera: camera.unwrap(),
        scene_tlas: scene_tlas.unwrap().into_boxed(),
        mesh: mesh.unwrap(),
        material: material.unwrap(),
        lighting: lighting.unwrap(),
        scene_ids,
      },
      std::mem::take(changed_s),
    )
  })
}
