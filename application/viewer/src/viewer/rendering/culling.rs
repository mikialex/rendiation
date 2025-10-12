use rendiation_frustum_culling::*;
use rendiation_occlusion_culling::GPUTwoPassOcclusionCulling;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_viewer_culling(
  cx: &mut QueryGPUHookCx,
  ndc: impl NDCSpaceMapper + Copy + Hash,
  enable_oc_support: bool,
  is_indirect: bool,
) -> Option<ViewerCulling> {
  let oc = if enable_oc_support && is_indirect {
    cx.scope(|cx| {
      let (_, oc) =
        cx.use_sharable_plain_state(|| GPUTwoPassOcclusionCulling::new(u16::MAX as usize));
      Some(oc.clone())
    })
  } else {
    None
  };

  let bounding_provider = if is_indirect {
    cx.scope(|cx| use_scene_model_device_world_bounding(cx).map(|b| Box::new(b) as Box<_>))
  } else {
    None
  };
  let camera_frustums = use_camera_gpu_frustum(cx, ndc);

  cx.when_render(|| ViewerCulling {
    oc,
    bounding_provider,
    frustums: camera_frustums.unwrap(),
  })
}

pub struct ViewerCulling {
  oc: Option<Arc<RwLock<GPUTwoPassOcclusionCulling>>>,
  bounding_provider: Option<Box<dyn DrawUnitWorldBoundingProvider>>,
  frustums: CameraGPUFrustums,
}

impl ViewerCulling {
  pub fn install_device_frustum_culler(
    &self,
    batch: &mut SceneModelRenderBatch,
    camera_gpu: &CameraGPU,
    camera: EntityHandle<SceneCameraEntity>,
  ) {
    if let SceneModelRenderBatch::Device(batch) = batch {
      let culler = GPUFrustumCuller {
        bounding_provider: self.bounding_provider.clone().unwrap(),
        frustum: self.frustums.get_gpu_frustum(camera),
        camera: camera_gpu.clone(),
      };

      batch.set_override_culler(culler);
    }
  }

  pub fn draw_with_oc_maybe_enabled(
    &self,
    ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    scene_pass_dispatcher: &dyn RenderComponent,
    camera_gpu: &CameraGPU,
    camera: EntityHandle<SceneCameraEntity>,
    preflight_content: &mut dyn FnMut(ActiveRenderPass) -> ActiveRenderPass,
    pass_base: RenderPassDescription,
    mut reorderable_batch: SceneModelRenderBatch,
  ) -> ActiveRenderPass {
    self.install_device_frustum_culler(&mut reorderable_batch, camera_gpu, camera);

    if let Some(oc) = &self.oc {
      oc.write().draw(
        ctx,
        camera.alloc_index(),
        &reorderable_batch.get_device_batch(None).unwrap(),
        pass_base,
        preflight_content,
        renderer.scene,
        camera_gpu,
        scene_pass_dispatcher,
        self.bounding_provider.clone().unwrap(),
        renderer.reversed_depth,
      )
    } else {
      let mut all_opaque_object = renderer.scene.make_scene_batch_pass_content(
        reorderable_batch,
        camera_gpu,
        scene_pass_dispatcher,
        ctx,
      );

      preflight_content(pass_base.render_ctx(ctx)).by(&mut all_opaque_object)
    }
  }
}
