use rendiation_lighting_shadow_map::*;

use crate::*;

pub fn use_cascade_shadow_map(
  cx: &mut QueryGPUHookCx,
  viewports: &[ViewerViewPort],
  ndc: ViewerNDC,
  shadow_pool_init_config: &MultiLayerTexturePackerConfig,
) -> Option<MultiCascadeShadowMapPreparer> {
  let camera_transform = cx.use_shared_dual_query(GlobalCameraTransformShare(ndc));

  let inputs = CascadeShadowMapSystemInputs {
    source_world: todo!(),
    source_proj: todo!(),
    size: todo!(),
    bias: todo!(),
    enabled: todo!(),
  };

  cx.when_render(|| {
    let per_camera = per_camera_per_viewport(viewports)
      .map(|cv| {
        let view_camera_proj = todo!();
        let view_camera_world = todo!();
        //
        generate_cascade_shadow_info(
          inputs,
          shadow_pool_init_config.init_size, // todo not supported grow
          view_camera_proj,
          view_camera_world,
          &ndc,
        );
      })
      .collect();

    MultiCascadeShadowMapPreparer { per_camera }
  })
}

type CascadeShadowGPUCacheShared = Arc<RwLock<CascadeShadowGPUCache>>;

pub struct MultiCascadeShadowMapPreparer {
  per_camera: FastHashMap<RawEntityHandle, (CascadeShadowPreparer, CascadeShadowGPUCacheShared)>,
}

pub struct MultiCascadeShadowMapData {
  per_camera: FastHashMap<RawEntityHandle, CascadeShadowMapComponent>,
}

impl MultiCascadeShadowMapData {
  pub fn get_shadow_accessor(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RandomAccessShadowProvider>> {
    // self
    //   .per_camera
    //   .get(&camera.into_raw())
    //   .map(|c| Box::new(c.clone()) as Box<dyn LightingComputeComponent>)
    todo!()
  }
}

pub trait RandomAccessShadowProvider: ShaderHashProvider {
  fn bind_shader(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RandomAccessShadowProviderInvocation>;
  fn bind_pass(&self, ctx: &mut BindingBuilder);
}

pub trait RandomAccessShadowProviderInvocation {
  fn get_shadow_by_light_id(&self, light_id: Node<u32>) -> Box<dyn ShadowOcclusionQuery>;
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
}
impl LightSystemSceneProvider for SceneDirectionalLightingCascadeShadowProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let shadow = self.shadow.get_shadow_accessor(camera);
    todo!()
  }
}
