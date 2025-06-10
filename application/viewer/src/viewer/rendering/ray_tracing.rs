use std::sync::Arc;

use rendiation_device_ray_tracing::GPUWaveFrontComputeRaytracingSystem;
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
  rtx: &RtxSystemCore,
  camera: Option<Box<dyn RtxCameraRenderImpl>>,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  tex: Option<GPUTextureBindingSystem>,
) -> Option<RayTracingRendererGroup> {
  let (cx, core) = cx.use_gpu_init(|gpu| {
    let rtx_backend_system = GPUWaveFrontComputeRaytracingSystem::new(gpu);
    RtxSystemCore::new(Box::new(rtx_backend_system))
  });

  let base = use_scene_rtx_renderer_base(cx, core, camera, materials, tex);
  let ao = use_rtx_ao_renderer(cx, rtx);
  let pt = use_rtx_pt_renderer(cx, rtx);

  cx.when_render(|| RayTracingRendererGroup {
    base: base.unwrap(),
    ao: ao.unwrap(),
    pt: pt.unwrap(),
  })
}

pub struct RayTracingRendererGroup {
  pub base: SceneRayTracingRendererBase,
  pub ao: SceneRayTracingAORenderer,
  pub pt: DeviceReferencePathTracingRenderer,
}
