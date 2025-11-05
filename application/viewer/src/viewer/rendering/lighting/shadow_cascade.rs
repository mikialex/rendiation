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

pub struct MultiCascadeShadowMapPreparer {
  per_camera_map: FastHashMap<RawEntityHandle, CascadeShadowPreparer>,
}

pub struct MultiCascadeShadowMapData {
  per_camera: FastHashMap<RawEntityHandle, usize>,
}

impl LightSystemSceneProvider for MultiCascadeShadowMapData {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
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
    todo!()
  }
}
