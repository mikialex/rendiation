use std::sync::Arc;

use rendiation_device_ray_tracing::GPUWaveFrontComputeRaytracingSystem;
use rendiation_scene_rendering_gpu_indirect::MeshGPUBindlessImpl;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingEffectMode {
  AO,
  ReferenceTracing,
}

pub fn use_viewer_rtx(
  cx: &mut impl QueryGPUHookCx,
  camera: Option<Box<dyn RtxCameraRenderImpl>>,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  mesh: Option<MeshGPUBindlessImpl>,
  tex: Option<GPUTextureBindingSystem>,
  request_reset_sample: bool,
) -> Option<(RayTracingRendererGroup, RtxSystemCore)> {
  let (cx, core) = cx.use_gpu_init(|gpu| {
    let rtx_backend_system = GPUWaveFrontComputeRaytracingSystem::new(gpu);
    RtxSystemCore::new(Box::new(rtx_backend_system))
  });

  let (cx, scope) = cx.use_begin_change_set_collect();
  let base = use_scene_rtx_renderer_base(cx, core, camera, mesh, materials, tex);
  let base_extra_changed = scope(cx);
  let request_reset_sample = request_reset_sample || base_extra_changed.unwrap_or(false);

  let ao = use_rtx_ao_renderer(cx, core, request_reset_sample);
  let pt = use_rtx_pt_renderer(cx, core, request_reset_sample);

  cx.when_render(|| {
    (
      RayTracingRendererGroup {
        base: base.unwrap(),
        ao: ao.unwrap(),
        pt: pt.unwrap(),
      },
      core.clone(),
    )
  })
}

pub struct RayTracingRendererGroup {
  pub base: SceneRayTracingRendererBase,
  pub ao: SceneRayTracingAORenderer,
  pub pt: DeviceReferencePathTracingRenderer,
}
