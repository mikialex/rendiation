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
) -> Option<RayTracingRendererGroup> {
  let base = use_scene_rtx_renderer_base(cx, camera, materials, tex);
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
