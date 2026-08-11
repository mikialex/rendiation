use crate::*;

pub fn use_cascade_shadow_map(
  cx: &mut QueryGPUHookCx,
  viewports: &[ViewerViewPort],
  ndc: ViewerNDC,
  shadow_pool_init_config: &MultiLayerTexturePackerConfig,
  lighting_sys: &LightSystem,
  lights: &Option<SharedLightUniformInfo<DirectionalLightUniform>>,
) -> Option<MultiCascadeShadowMapPreparer> {
  let camera_transform = cx
    .use_shared_dual_query_view(GlobalCameraTransformShare(ndc))
    .use_assure_result(cx);

  cx.next_scope_index();
  let maps = per_camera_per_viewport(viewports, false)
    .map(|cv| {
      let (cache, shadow_map) = cx.keyed_scope(&cv.camera, |cx| {
        let cache = cx
          .use_plain_state_default_cloned::<CascadeShadowGPUCacheShared>()
          .1;
        let shadow_map = use_shadow_map(
          cx,
          lighting_sys,
          ndc.enable_reverse_z,
          Some(shadow_pool_init_config.init_size),
        );
        (cache, shadow_map)
      });
      (cv.camera, (cache, shadow_map))
    })
    .collect::<FastHashMap<_, _>>();

  let source_world = use_global_node_world_mat_view(cx).use_assure_result(cx);

  cx.when_render(|| {
    let lights = lights.as_ref().unwrap().read();
    let mapping = &lights.allocation_info;

    let light_ref_node = get_db_view::<DirectionalRefNode>();
    let shadow_enabled = get_db_view::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>();
    let shadow_map_size =
      get_db_view::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>();
    let shadow_bias = get_db_view::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>();
    let shadow_proj = get_db_view::<DirectionLightShadowBound>();
    let source_world = source_world.expect_resolve_stage();

    let cascade_info_access = |light_id: RawEntityHandle| -> Option<CascadeShadowMapLightInput> {
      let enabled = shadow_enabled.access(&light_id).unwrap();
      if !enabled {
        return None;
      }
      let node = light_ref_node.access(&light_id).unwrap()?;
      let source_world = source_world.access(&node)?;
      let size = shadow_map_size.access(&light_id).unwrap();
      let bias = shadow_bias.access(&light_id).unwrap();
      let orth = shadow_proj
        .access(&light_id)
        .unwrap()
        .unwrap_or(DEFAULT_DIR_PROJ);

      Some(CascadeShadowMapLightInput {
        source_world,
        shadow_near_far: (orth.near, orth.far),
        size: Size::from_u32_pair_min_one(size.into()),
        bias: bias.into(),
        shadow_enabled: true,
      })
    };

    let camera_transform = camera_transform.expect_resolve_stage();

    let per_camera = per_camera_per_viewport(viewports, false)
      .map(|cv| {
        let transform = camera_transform.access(&cv.camera.into_raw()).unwrap();
        let view_camera_proj = transform.projection;
        let view_camera_world = transform.world;
        //
        let info = generate_cascade_shadow_info(
          &cascade_info_access,
          shadow_pool_init_config.init_size,
          view_camera_proj,
          view_camera_world,
          &ndc,
          lighting_sys.cascade_shadow_split_linear_log_blend_ratio,
          &mapping,
        );
        let (cache, shadow_map) = maps.get(&cv.camera).unwrap();
        (
          cv.camera,
          PerCameraCascadeShadowPreparer {
            updater: info,
            cache: cache.clone(),
            shadow_map: shadow_map.clone(),
          },
        )
      })
      .collect();

    MultiCascadeShadowMapPreparer { per_camera }
  })
}

type CascadeShadowGPUCacheShared = Arc<RwLock<CascadeShadowGPUCache>>;

pub struct PerCameraCascadeShadowPreparer {
  updater: CascadeShadowPreparer,
  cache: CascadeShadowGPUCacheShared,
  shadow_map: Box<dyn AbstractShadowMapGPUData>,
}

pub struct MultiCascadeShadowMapPreparer {
  per_camera: FastHashMap<EntityHandle<SceneCameraEntity>, PerCameraCascadeShadowPreparer>,
}

impl MultiCascadeShadowMapPreparer {
  pub fn update(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
  ) -> MultiCascadeShadowMapData {
    let per_camera = self
      .per_camera
      .into_iter()
      .map(|(k, mut per_camera)| {
        let mut cache = per_camera.cache.write();
        let gpu_data = per_camera.updater.update_shadow_maps(
          &mut cache,
          frame_ctx,
          per_camera.shadow_map.as_mut(),
          draw,
        );
        (
          k,
          PerCameraCascadeShadowData {
            gpu_data,
            shadow_map: per_camera.shadow_map,
          },
        )
      })
      .collect();
    MultiCascadeShadowMapData { per_camera }
  }
}

pub struct PerCameraCascadeShadowData {
  pub gpu_data: CascadeShadowGPUData,
  pub shadow_map: Box<dyn AbstractShadowMapGPUData>,
}

pub struct MultiCascadeShadowMapData {
  pub per_camera: FastHashMap<EntityHandle<SceneCameraEntity>, PerCameraCascadeShadowData>,
}
