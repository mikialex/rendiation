use std::sync::Arc;

use rendiation_device_ray_tracing::{
  GPURaytracingSystem, GPUWaveFrontComputeRaytracingSystem, WaveFrontTracingBaseProvider,
};
use rendiation_scene_rendering_gpu_ray_tracing::*;

use crate::*;

pub fn use_raytracing_rendering(cx: &mut Viewer3dRenderingCx) {
  let (cx, rtx_core) = cx.use_gpu_state(|gpu| {
    let sys = Box::new(GPUWaveFrontComputeRaytracingSystem::new(gpu));
    RtxSystemCore::new(sys)
  });
  let (cx, mode) = cx.use_plain_state_init(&RayTracingEffectMode::ReferenceTracing);

  let camera = cx.on_render(|cx| {
    // get camera
  });

  let materials = cx.on_render(|cx| {
    // get materials
  });

  let texture = cx.on_render(|cx| {
    // get textures
  });

  let base = use_scene_rtx_renderer_base(cx, todo!(), todo!(), todo!());

  if let Some(ao) = use_rtx_ao_renderer(cx, todo!()) {
    if base.unwrap().any_changed {
      ao.reset_sample();
    }

    let ao_result = ao.render(
      &mut ctx,
      todo!(),
      &mut rtx_renderer.base,
      content.scene,
      content.main_camera,
    );

    pass("copy rtx ao into final target")
      .with_color(target, store_full_frame())
      .render_ctx(&mut ctx)
      .by(&mut copy_frame(RenderTargetView::from(ao_result), None));
  }

  //
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingEffectMode {
  AO,
  ReferenceTracing,
}

// pub struct RayTracingSystemGroup {
//   pub base: RayTracingSystemBase,
//   pub ao: RayTracingAORenderSystem,
//   pub pt: DeviceReferencePathTracingSystem,
// }

// impl RayTracingSystemGroup {
//   pub fn new(
//     rtx: &RtxSystemCore,
//     gpu: &GPU,
//     camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
//   ) -> Self {
//     Self {
//       base: RayTracingSystemBase::new(rtx, gpu, camera_source),
//       ao: RayTracingAORenderSystem::new(rtx, gpu),
//       pt: DeviceReferencePathTracingSystem::new(rtx, gpu),
//     }
//   }
// }

// impl QueryBasedFeature<RayTracingFeatureGroup> for RayTracingSystemGroup {
//   type Context = GPU;
//   fn register(&mut self, qcx: &mut ReactiveQueryCtx, gpu: &GPU) {
//     self.base.register(qcx, gpu);
//     self.ao.register(qcx, gpu);
//     self.pt.register(qcx, gpu);
//   }

//   fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
//     self.base.deregister(qcx);
//     self.ao.deregister(qcx);
//     self.pt.deregister(qcx);
//   }

//   fn create_impl(&self, cx: &mut QueryResultCtx) -> RayTracingFeatureGroup {
//     RayTracingFeatureGroup {
//       base: self.base.create_impl(cx),
//       ao: self.ao.create_impl(cx),
//       pt: self.pt.create_impl(cx),
//     }
//   }
// }

// pub struct RayTracingFeatureGroup {
//   pub base: SceneRayTracingRendererBase,
//   pub ao: SceneRayTracingAORenderer,
//   pub pt: DeviceReferencePathTracingRenderer,
// }
