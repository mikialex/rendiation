use rendiation_lighting_shadow_map::*;

use crate::*;

pub fn use_cascade_shadow_map(
  cx: &mut QueryGPUHookCx,
  viewports: &[ViewerViewPort],
  shadow_pool_init_config: &MultiLayerTexturePackerConfig,
) -> Option<MultiCascadeShadowMapPreparer> {
  //   let per_camera = viewports
  //     .iter()
  //     .map(|v| {
  //       //  cx.keyed_scope(v.id, || {
  //       //       //
  //       //     })
  //       todo!()
  //     })
  //     .collect();

  //   generate_cascade_shadow_info();

  todo!()
}

type CascadeShadowGPUCacheShared = Arc<RwLock<CascadeShadowGPUCache>>;

pub struct MultiCascadeShadowMapPreparer {
  per_camera: FastHashMap<RawEntityHandle, (CascadeShadowPreparer, CascadeShadowGPUCacheShared)>,
}

pub struct MultiCascadeShadowMapData {
  per_camera: FastHashMap<RawEntityHandle, CascadeShadowMapComponent>,
}

impl LightSystemSceneProvider for MultiCascadeShadowMapData {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    // self
    //   .per_camera
    //   .get(&camera.into_raw())
    //   .map(|c| Box::new(c.clone()) as Box<dyn LightingComputeComponent>)
    todo!()
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
