use rendiation_lighting_punctual::DirectionalShaderInfo;
use rendiation_lighting_shadow_map::*;

use crate::{viewer::rendering::lighting::punctual::DEFAULT_DIR_PROJ, *};

pub fn use_cascade_shadow_map(
  cx: &mut QueryGPUHookCx,
  viewports: &[ViewerViewPort],
  ndc: ViewerNDC,
  shadow_pool_init_config: &MultiLayerTexturePackerConfig,
  split_linear_log_blend_ratio: f32,
) -> Option<MultiCascadeShadowMapPreparer> {
  let camera_transform = cx
    .use_shared_dual_query(GlobalCameraTransformShare(ndc))
    .use_assure_result(cx);

  let maps = per_camera_per_viewport(viewports)
    .map(|cv| {
      let cache = cx.keyed_scope(&cv.camera, |cx| {
        cx.use_plain_state_default_cloned::<CascadeShadowGPUCacheShared>()
          .1
      });
      (cv.camera, cache)
    })
    .collect::<FastHashMap<_, _>>();

  let source_world = use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<DirectionalRefNode>(), cx)
    .use_assure_result(cx);

  cx.when_render(|| {
    let enabled =
      get_db_view_no_generation_check::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>()
        .into_boxed();
    let bias =
      get_db_view_no_generation_check::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>()
        .map_value(|bias| bias.into())
        .into_boxed();
    let size = get_db_view_no_generation_check::<
      BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>,
    >()
    .map_value(|size| Size::from_u32_pair_min_one(size.into()))
    .into_boxed();

    let source_world = source_world
      .expect_resolve_stage()
      .view()
      .skip_generation_check::<DirectionalLightEntity>()
      .into_boxed();

    let source_proj = get_db_view_no_generation_check::<DirectionLightShadowBound>()
      .map_value(move |orth| {
        orth
          .unwrap_or(DEFAULT_DIR_PROJ)
          .compute_projection_mat(&ndc)
      })
      .into_boxed();

    let inputs = CascadeShadowMapSystemInputs {
      source_world,
      source_proj,
      size,
      bias,
      enabled,
    };

    let camera_transform = camera_transform.expect_resolve_stage();

    let per_camera = per_camera_per_viewport(viewports)
      .map(|cv| {
        let transform = camera_transform.view.access(&cv.camera.into_raw()).unwrap();
        let view_camera_proj = transform.projection;
        let view_camera_world = transform.world;
        //
        let info = generate_cascade_shadow_info(
          &inputs,
          shadow_pool_init_config.init_size, // todo not supported grow
          view_camera_proj,
          view_camera_world,
          &ndc,
          split_linear_log_blend_ratio,
        );
        let map = maps.get(&cv.camera).unwrap().clone();
        (cv.camera, (info, map))
      })
      .collect();

    MultiCascadeShadowMapPreparer { per_camera }
  })
}

type CascadeShadowGPUCacheShared = Arc<RwLock<CascadeShadowGPUCache>>;

pub struct MultiCascadeShadowMapPreparer {
  per_camera: FastHashMap<
    EntityHandle<SceneCameraEntity>,
    (CascadeShadowPreparer, CascadeShadowGPUCacheShared),
  >,
}

pub struct MultiCascadeShadowMapData {
  per_camera: FastHashMap<EntityHandle<SceneCameraEntity>, CascadeShadowMapComponent>,
}

impl MultiCascadeShadowMapData {
  pub fn get_shadow_accessor(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<CascadeShadowMapComponent> {
    let cascade_info = self.per_camera.get(&camera)?.clone();
    Some(cascade_info)
  }
}

impl MultiCascadeShadowMapPreparer {
  pub fn update(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> MultiCascadeShadowMapData {
    let per_camera = self
      .per_camera
      .into_iter()
      .map(|(k, (updater, map))| {
        let mut map = map.write();
        let com = updater.update_shadow_maps(&mut map, frame_ctx, draw, reversed_depth);
        (k, com)
      })
      .collect();
    MultiCascadeShadowMapData { per_camera }
  }
}

pub struct SceneDirectionalLightingCascadeShadowProvider {
  pub shadow: MultiCascadeShadowMapData,
  pub light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
}
impl LightSystemSceneProvider for SceneDirectionalLightingCascadeShadowProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let shadow = self.shadow.get_shadow_accessor(camera)?;

    let lights_iter_compute = AbstractShaderBindingIterSourceHelperMap::new(
      self.light.clone(),
      shadow,
      |(light_id, light_uniform): (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightUniform>),
       shadow: &CascadeShadowMapInvocation| {
        let light_uniform = light_uniform.load().expand();
        let light = ENode::<DirectionalShaderInfo> {
          illuminance: light_uniform.illuminance,
          direction: light_uniform.direction,
        }
        .construct();
        let shadow = ShadowRandomAccessed {
          shadow: Arc::new(shadow.clone()),
          light_id,
        };
        ShadowedPunctualLighting { light, shadow }
      },
    );

    let com = ArrayLights(lights_iter_compute);
    Some(Box::new(com))
  }
}

pub struct LightWithRandomAccessShadow<L, S> {
  pub light: L,
  pub shadow: S,
}
