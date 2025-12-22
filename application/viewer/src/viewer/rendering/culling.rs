use rendiation_frustum_culling::*;
use rendiation_occlusion_culling::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_viewer_culling(
  cx: &mut QueryGPUHookCx,
  ndc: impl NDCSpaceMapper + Copy + Hash,
  config: &ViewerCullingConfig,
  is_indirect: bool,
  viewports: &[ViewerViewPort],
) -> Option<ViewerCulling> {
  let oc_states = if config.enable_indirect_occlusion_culling && is_indirect {
    cx.scope(|cx| {
      cx.next_key_scope_root();
      let maps = per_camera_per_viewport(viewports, true)
        .map(|cv| {
          let cache = cx.keyed_scope(&cv.camera, |cx| {
            let (_, oc) = cx.use_sharable_plain_state(|| {
              GPUTwoPassOcclusionCulling::new(
                config.occlusion_culling_max_scene_model_count as usize,
                cx.gpu,
              )
            });
            oc
          });
          (cv.camera, cache)
        })
        .collect::<FastHashMap<_, _>>();

      Some(maps)
    })
  } else {
    None
  };

  let sm_world_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding)
    .use_assure_result(cx);

  let bounding_provider = if is_indirect {
    cx.scope(|cx| {
      let bounding = cx.use_shared_dual_query(SceneModelWorldBounding);
      use_scene_model_device_world_bounding(cx, bounding).map(|b| Box::new(b) as Box<_>)
    })
  } else {
    None
  };
  let camera_frustums = use_camera_gpu_frustum(cx, ndc);

  cx.when_render(|| ViewerCulling {
    oc: oc_states.map(|oc_states| ViewerOcclusionCulling {
      oc_states,
      enable_debug_cull_result: config.enable_debug_occlusion_culling_result,
      debug_culled_result: Default::default(),
    }),
    bounding_provider,
    sm_world_bounding: sm_world_bounding
      .expect_resolve_stage()
      .mark_entity_type()
      .into_boxed(),
    frustums: camera_frustums.unwrap(),
    enable_frustum_culling: config.enable_frustum_culling,
  })
}

pub struct ViewerOcclusionCulling {
  pub oc_states:
    FastHashMap<EntityHandle<SceneCameraEntity>, Arc<RwLock<GPUTwoPassOcclusionCulling>>>,
  pub enable_debug_cull_result: bool,
  pub debug_culled_result:
    FastHashMap<EntityHandle<SceneCameraEntity>, GPUTwoPassOcclusionCullingDebugDrawBatchResult>,
}

pub struct ViewerCulling {
  oc: Option<ViewerOcclusionCulling>,
  bounding_provider: Option<Box<dyn DrawUnitWorldBoundingProvider>>,
  sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  frustums: CameraGPUFrustums,
  enable_frustum_culling: bool,
}

impl ViewerCulling {
  pub fn install_frustum_culler(
    &self,
    batch: &mut SceneModelRenderBatch,
    camera_gpu: &CameraGPU,
    camera: EntityHandle<SceneCameraEntity>,
  ) {
    if !self.enable_frustum_culling {
      return;
    }
    match batch {
      SceneModelRenderBatch::Device(batch) => {
        let culler = GPUFrustumCuller {
          bounding_provider: self.bounding_provider.clone().unwrap(),
          frustum: self.frustums.get_gpu_frustum(camera),
          camera: camera_gpu.clone(),
        };

        batch.set_override_culler(culler);
      }
      SceneModelRenderBatch::Host(host_render_batch) => {
        *host_render_batch = Box::new(HostFrustumCulling {
          inner: host_render_batch.clone(),
          sm_world_bounding: self.sm_world_bounding.clone(),
          frustum: self.frustums.get_frustum(camera),
        })
      }
    }
  }

  pub fn draw_with_oc_maybe_enabled(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    scene_pass_dispatcher: &dyn RenderComponent,
    camera_gpu: &CameraGPU,
    viewport: &ViewerViewPort,
    preflight_content: &mut dyn FnMut(ActiveRenderPass) -> ActiveRenderPass,
    pass_base: RenderPassDescription,
    mut reorderable_batch: SceneModelRenderBatch,
  ) -> ActiveRenderPass {
    let camera = viewport.camera;
    self.install_frustum_culler(&mut reorderable_batch, camera_gpu, camera);

    if let Some(oc) = &mut self.oc {
      ctx.scope(|ctx| {
        if let Some(oc_debug_camera) = viewport.debug_camera_for_view_related {
          if let Some(previous_oc_batch) = oc.debug_culled_result.get(&oc_debug_camera) {
            return ctx.scope(|ctx| {
              let mut drawn_occluder = renderer.scene.make_scene_batch_pass_content(
                SceneModelRenderBatch::Device(previous_oc_batch.drawn_occluder.clone()),
                camera_gpu,
                scene_pass_dispatcher,
                ctx,
              );

              let mut drawn_not_occluded = renderer.scene.make_scene_batch_pass_content(
                SceneModelRenderBatch::Device(previous_oc_batch.drawn_not_occluded.clone()),
                camera_gpu,
                scene_pass_dispatcher,
                ctx,
              );

              pass_base
                .with_name("occlusion-culling-debug-for-other-view")
                .render_ctx(ctx)
                .by(&mut drawn_occluder)
                .by(&mut drawn_not_occluded)
            });
          } else {
            log::warn!("the oc debug info can not be found, make sure the debug is enabled or adjust the viewport rendering order to make sure the oc is drawn before the debug camera");
          }
        }

        let oc_state = oc.oc_states.get(&camera).unwrap();
        let (pass, debug) = oc_state.write().draw(
          ctx,
          &reorderable_batch.get_device_batch().unwrap(),
          pass_base,
          preflight_content,
          renderer.scene,
          camera_gpu,
          scene_pass_dispatcher,
          self.bounding_provider.clone().unwrap(),
          renderer.reversed_depth,
          oc.enable_debug_cull_result,
        );

        if let Some(debug) = debug {
          oc.debug_culled_result.insert(camera, debug);
        }

      pass
      })
    } else {
      ctx.scope(|ctx| {
        let mut all_opaque_object = renderer.scene.make_scene_batch_pass_content(
          reorderable_batch,
          camera_gpu,
          scene_pass_dispatcher,
          ctx,
        );

        preflight_content(pass_base.render_ctx(ctx)).by(&mut all_opaque_object)
      })
    }
  }
}
