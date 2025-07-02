use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub fn use_directional_light_uniform(
  qcx: &mut impl QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
) -> Option<SceneDirectionalLightingPreparer> {
  let (qcx, shadow) = qcx.use_gpu_init(|gpu| {
    let source_proj = global_watch()
      .watch_untyped_key::<DirectionLightShadowBound>()
      .collective_map(move |orth| {
        orth
          .unwrap_or(OrthographicProjection {
            left: -20.,
            right: 20.,
            top: 20.,
            bottom: -20.,
            near: 0.,
            far: 1000.,
          })
          .compute_projection_mat(&ndc)
      })
      .into_boxed();

    let source_world = scene_node_derive_world_mat()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
      .untyped_entity_handle()
      .into_boxed();

    basic_shadow_map_uniform_takeable(
      ShadowMapSystemInputs {
        source_world,
        source_proj,
        size: global_watch()
          .watch_untyped_key::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>()
          .collective_map(|size| Size::from_u32_pair_min_one(size.into()))
          .into_boxed(),
        bias: global_watch()
          .watch_untyped_key::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>()
          .collective_map(|v| v.into())
          .into_boxed(),
        enabled: global_watch()
          .watch_untyped_key::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>()
          .into_boxed(),
      },
      *init_config,
      gpu,
    )
  });

  let shadow_uniform = qcx.use_uniform_array_buffers(|_| shadow.1.take().unwrap());

  qcx
    .use_uniform_array_buffers(directional_uniform_array)
    .map(|light| SceneDirectionalLightingPreparer {
      shadow: shadow.0.clone(),
      light,
      info: shadow_uniform.unwrap(),
    })
}

pub struct SceneDirectionalLightingPreparer {
  pub shadow: Arc<RwLock<BasicShadowMapSystem>>,
  pub light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
  pub info: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
}

impl SceneDirectionalLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> SceneDirectionalLightingProvider {
    noop_ctx!(cx);
    let shadow_map_atlas =
      self
        .shadow
        .write()
        .update_shadow_maps(cx, frame_ctx, draw, reversed_depth);

    SceneDirectionalLightingProvider {
      light: self.light,
      shadow: BasicShadowMapComponent {
        shadow_map_atlas,
        info: self.info,
        reversed_depth,
      },
    }
  }
}

pub struct SceneDirectionalLightingProvider {
  light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
  shadow: BasicShadowMapComponent,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      LightAndShadowCombinedSource(self.light.clone(), self.shadow.clone()),
      |((_, light), shadow): (
        (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightUniform>),
        BasicShadowMapSingleInvocation,
      )| {
        let light_uniform = light.load().expand();
        let light = ENode::<DirectionalShaderInfo> {
          illuminance: light_uniform.illuminance,
          direction: light_uniform.direction,
        }
        .construct();
        ShadowedPunctualLighting { light, shadow }
      },
    );
    Some(Box::new(com))
  }
}

pub fn use_scene_point_light_uniform(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<ScenePointLightingProvider> {
  qcx
    .use_uniform_array_buffers(point_uniform_array)
    .map(|uniform| ScenePointLightingProvider { uniform })
}

pub struct ScenePointLightingProvider {
  uniform: UniformBufferDataView<Shader140Array<PointLightUniform, 8>>,
}

impl LightSystemSceneProvider for ScenePointLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.uniform.clone(),
      |(_, light_uniform): (Node<u32>, ShaderReadonlyPtrOf<PointLightUniform>)| {
        let light_uniform = light_uniform.load().expand();
        ENode::<PointLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          cutoff_distance: light_uniform.cutoff_distance,
        }
        .construct()
      },
    );
    Some(Box::new(com))
  }
}

pub fn use_scene_spot_light_uniform(
  qcx: &mut impl QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
) -> Option<SceneSpotLightingPreparer> {
  let (qcx, shadow) = qcx.use_gpu_init(|gpu| {
    let source_proj = global_watch()
      .watch_untyped_key::<SpotLightHalfConeAngle>()
      .collective_map(move |half_cone_angle| {
        PerspectiveProjection {
          near: 0.1,
          far: 2000.,
          fov: Deg::from_rad(half_cone_angle * 2.),
          aspect: 1.,
        }
        .compute_projection_mat(&ndc)
      })
      .into_boxed();

    let source_world = scene_node_derive_world_mat()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
      .untyped_entity_handle()
      .into_boxed();

    basic_shadow_map_uniform_takeable(
      ShadowMapSystemInputs {
        source_proj,
        source_world,
        size: global_watch()
          .watch_untyped_key::<BasicShadowMapResolutionOf<SpotLightBasicShadowInfo>>()
          .collective_map(|size| Size::from_u32_pair_min_one(size.into()))
          .into_boxed(),
        bias: global_watch()
          .watch_untyped_key::<BasicShadowMapBiasOf<SpotLightBasicShadowInfo>>()
          .collective_map(|v| v.into())
          .into_boxed(),
        enabled: global_watch()
          .watch_untyped_key::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>()
          .into_boxed(),
      },
      *init_config,
      gpu,
    )
  });

  let shadow_uniform = qcx.use_uniform_array_buffers(|_| shadow.1.take().unwrap());

  qcx
    .use_uniform_array_buffers(spot_uniform_array)
    .map(|light| SceneSpotLightingPreparer {
      shadow: shadow.0.clone(),
      light,
      info: shadow_uniform.unwrap(),
    })
}

pub struct SceneSpotLightingPreparer {
  pub shadow: Arc<RwLock<BasicShadowMapSystem>>,
  pub light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
  pub info: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
}

impl SceneSpotLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> SceneSpotLightingProvider {
    noop_ctx!(cx);
    let shadow_map_atlas =
      self
        .shadow
        .write()
        .update_shadow_maps(cx, frame_ctx, draw, reversed_depth);

    SceneSpotLightingProvider {
      light: self.light,
      shadow: BasicShadowMapComponent {
        shadow_map_atlas,
        info: self.info,
        reversed_depth,
      },
    }
  }
}

pub struct SceneSpotLightingProvider {
  light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
  shadow: BasicShadowMapComponent,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      LightAndShadowCombinedSource(self.light.clone(), self.shadow.clone()),
      |((_, light), shadow): (
        (Node<u32>, ShaderReadonlyPtrOf<SpotLightUniform>),
        BasicShadowMapSingleInvocation,
      )| {
        let light_uniform = light.load().expand();
        let light = ENode::<SpotLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          direction: light_uniform.direction,
          cutoff_distance: light_uniform.cutoff_distance,
          half_cone_cos: light_uniform.half_cone_cos,
          half_penumbra_cos: light_uniform.half_penumbra_cos,
        }
        .construct();
        ShadowedPunctualLighting { light, shadow }
      },
    );

    Some(Box::new(com))
  }
}

pub fn basic_shadow_map_uniform_takeable(
  inputs: ShadowMapSystemInputs,
  config: MultiLayerTexturePackerConfig,
  gpu_ctx: &GPU,
) -> (
  Arc<RwLock<BasicShadowMapSystem>>,
  Option<UniformArrayUpdateContainer<BasicShadowMapInfo, 8>>,
) {
  let (map, uniform) = basic_shadow_map_uniform(inputs, config, gpu_ctx);
  let map = Arc::new(RwLock::new(map));
  (map, Some(uniform))
}
