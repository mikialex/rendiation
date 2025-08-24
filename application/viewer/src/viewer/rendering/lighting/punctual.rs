use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_directional_light_uniform(
  qcx: &mut QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
) -> Option<SceneDirectionalLightingPreparer> {
  let source_world =
    use_global_node_world_mat(qcx).fanout(qcx.use_db_rev_ref_tri_view::<DirectionalRefNode>());

  let source_proj = qcx
    .use_dual_query::<DirectionLightShadowBound>()
    .dual_query_map(move |orth| {
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
    });

  let size = qcx
    .use_dual_query::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>()
    .into_delta_change()
    .map_changes(|size| Size::from_u32_pair_min_one(size.into()));

  let bias = qcx
    .use_changes::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>()
    .map_changes(|v| v.into());

  let enabled = qcx.use_changes::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>();

  let shadow = use_basic_shadow_map_uniform(
    qcx,
    source_world,
    source_proj,
    size,
    bias,
    enabled,
    *init_config,
  );

  let light = use_directional_uniform_array(qcx);

  qcx.when_render(|| {
    let (updater, shadow_uniform) = shadow.unwrap();
    SceneDirectionalLightingPreparer {
      shadow: updater,
      light,
      info: shadow_uniform,
    }
  })
}

pub struct SceneDirectionalLightingPreparer {
  pub shadow: BasicShadowMapPreparer,
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
    let shadow_map_atlas = self
      .shadow
      .update_shadow_maps(frame_ctx, draw, reversed_depth);

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
  qcx: &mut QueryGPUHookCx,
) -> Option<ScenePointLightingProvider> {
  let uniform = use_point_uniform_array(qcx);
  qcx.when_render(|| ScenePointLightingProvider { uniform })
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
          position: hpt_uniform_to_hpt(light_uniform.position),
          cutoff_distance: light_uniform.cutoff_distance,
        }
        .construct()
      },
    );
    Some(Box::new(com))
  }
}

pub fn use_scene_spot_light_uniform(
  qcx: &mut QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
) -> Option<SceneSpotLightingPreparer> {
  let source_world =
    use_global_node_world_mat(qcx).fanout(qcx.use_db_rev_ref_tri_view::<SpotLightRefNode>());

  let source_proj = qcx
    .use_dual_query::<SpotLightHalfConeAngle>()
    .dual_query_map(move |half_cone_angle| {
      PerspectiveProjection {
        near: 0.1,
        far: 2000.,
        fov: Deg::from_rad(half_cone_angle * 2.),
        aspect: 1.,
      }
      .compute_projection_mat(&ndc)
    });

  let size = qcx
    .use_dual_query::<BasicShadowMapResolutionOf<SpotLightBasicShadowInfo>>()
    .into_delta_change()
    .map_changes(|size| Size::from_u32_pair_min_one(size.into()));

  let bias = qcx
    .use_changes::<BasicShadowMapBiasOf<SpotLightBasicShadowInfo>>()
    .map_changes(|v| v.into());

  let enabled = qcx.use_changes::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>();

  let shadow = use_basic_shadow_map_uniform(
    qcx,
    source_world,
    source_proj,
    size,
    bias,
    enabled,
    *init_config,
  );

  let light = use_spot_uniform_array(qcx);

  qcx.when_render(|| {
    let (updater, shadow_uniform) = shadow.unwrap();
    SceneSpotLightingPreparer {
      shadow: updater,
      light,
      info: shadow_uniform,
    }
  })
}

pub struct SceneSpotLightingPreparer {
  pub shadow: BasicShadowMapPreparer,
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
    let shadow_map_atlas = self
      .shadow
      .update_shadow_maps(frame_ctx, draw, reversed_depth);

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
          position: hpt_uniform_to_hpt(light_uniform.position),
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
