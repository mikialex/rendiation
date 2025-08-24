use rendiation_frustum_culling::*;
use rendiation_occlusion_culling::GPUTwoPassOcclusionCulling;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_viewer_culling(
  cx: &mut QueryGPUHookCx,
  ndc: impl NDCSpaceMapper + Copy,
  enable_oc_support: bool,
) -> Option<ViewerCulling> {
  let oc = if enable_oc_support {
    cx.scope(|cx| {
      let (_, oc) = cx.use_gpu_init(|_| {
        let oc = GPUTwoPassOcclusionCulling::new(u16::MAX as usize);
        Arc::new(RwLock::new(oc))
      });
      Some(oc.clone())
    })
  } else {
    None
  };

  let bounding_provider = use_scene_model_device_world_bounding(cx).map(|b| Box::new(b) as Box<_>);
  let camera_frustums = use_camera_gpu_frustum(cx, ndc);

  cx.when_render(|| ViewerCulling {
    oc,
    bounding_provider: bounding_provider.unwrap(),
    frustums: camera_frustums.unwrap(),
  })
}

pub struct ViewerCulling {
  oc: Option<Arc<RwLock<GPUTwoPassOcclusionCulling>>>,
  bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
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
        bounding_provider: self.bounding_provider.clone(),
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
        self.bounding_provider.clone(),
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
