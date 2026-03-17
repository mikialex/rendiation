use std::sync::Arc;

use rendiation_device_ray_tracing::GPUWaveFrontComputeRaytracingSystem;
use rendiation_scene_rendering_gpu_indirect::MeshGPUBindlessImpl;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingEffectMode {
  AO,
  ReferenceTracing,
}

pub fn use_viewer_rtx(
  cx: &mut QueryGPUHookCx,
  camera: Option<Box<dyn RtxCameraRenderImpl>>,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  mesh: Option<MeshGPUBindlessImpl>,
  tex: Option<GPUTextureBindingSystem>,
  request_reset_sample: bool,
) -> Option<(RayTracingRendererGroup, RtxSystemCore)> {
  let (cx, core) = cx.use_gpu_init(|gpu, alloc| {
    let rtx_backend_system = GPUWaveFrontComputeRaytracingSystem::new(gpu, alloc);
    RtxSystemCore::new(Box::new(rtx_backend_system))
  });

  let mesh_input = viewer_mesh_input(cx);
  let base = use_scene_rtx_renderer_base(cx, core, camera, mesh, materials, tex, mesh_input);

  let ao = use_scene_ao_sbt(cx, core);
  let pt = use_rtx_pt_sbt(cx, core);

  cx.when_render(|| {
    let mut base = base.unwrap();
    base.1 |= request_reset_sample;
    (
      RayTracingRendererGroup {
        base,
        ao: ao.unwrap(),
        pt: pt.unwrap(),
      },
      core.clone(),
    )
  })
}

pub struct RayTracingRendererGroup {
  pub base: (SceneRayTracingRendererBase, bool),
  pub ao: (GPUSbt, bool),
  pub pt: (GPUSbt, bool),
}
