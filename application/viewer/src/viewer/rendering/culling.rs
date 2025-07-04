use rendiation_occlusion_culling::GPUTwoPassOcclusionCulling;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub fn use_camera_gpu_frustum(
  qcx: &mut impl QueryGPUHookCx,
  camera_source: &RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
) -> Option<CameraGPUFrustums> {
  qcx
    .use_uniform_buffers(|source, cx| {
      let c = camera_source
        .clone()
        .collective_map(|transform| {
          let arr = Frustum::new_from_matrix(transform.view_projection)
            .planes
            .map(|p| Vec4::new(p.normal.x, p.normal.y, p.normal.z, p.constant).into_f32());

          Shader140Array::<Vec4<f32>, 6>::from_slice_clamp_or_default(&arr);
        })
        .into_query_update_uniform(0, cx);

      source.with_source(c)
    })
    .map(|frustums| CameraGPUFrustums { frustums })
}

type CameraGPUFrustumsUniform =
  UniformUpdateContainer<EntityHandle<SceneCameraEntity>, Shader140Array<Vec4<f32>, 6>>;

pub struct CameraGPUFrustums {
  frustums: LockReadGuardHolder<CameraGPUFrustumsUniform>,
}

impl CameraGPUFrustums {
  pub fn get_gpu_frustum(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> UniformBufferDataView<Shader140Array<Vec4<f32>, 6>> {
    self.frustums.get(&camera).unwrap().clone()
  }
}

// todo, disable resource if not indirect rendering
pub fn use_viewer_culling(
  cx: &mut impl QueryGPUHookCx,
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

  cx.when_render(|| ViewerCulling {
    oc,
    bounding_provider: bounding_provider.unwrap(),
  })
}

pub struct ViewerCulling {
  oc: Option<Arc<RwLock<GPUTwoPassOcclusionCulling>>>,
  bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
}

impl ViewerCulling {
  pub fn draw_with_oc_maybe_enabled(
    &self,
    ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    scene_pass_dispatcher: &dyn RenderComponent,
    camera_gpu: &CameraGPU,
    camera: EntityHandle<SceneCameraEntity>,
    preflight_content: &mut dyn FnMut(ActiveRenderPass) -> ActiveRenderPass,
    pass_base: RenderPassDescription,
    reorderable_batch: SceneModelRenderBatch,
  ) -> ActiveRenderPass {
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
