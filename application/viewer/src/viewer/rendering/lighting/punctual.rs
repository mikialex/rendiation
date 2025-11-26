use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub const DEFAULT_DIR_PROJ: OrthographicProjection<f32> = OrthographicProjection {
  left: -20.,
  right: 20.,
  top: 20.,
  bottom: -20.,
  near: 0.,
  far: 1000.,
};

pub fn use_directional_light_uniform(
  cx: &mut QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  viewports: &[ViewerViewPort],
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
) -> Option<SceneDirectionalLightingPreparer> {
  let shadow = if lighting_sys.enable_shadow {
    cx.scope(|cx| {
      let shadow = if lighting_sys.use_cascade_shadowmap_for_directional_lights {
        cx.scope(|cx| {
          use_cascade_shadow_map(
            cx,
            viewports,
            ndc,
            init_config,
            lighting_sys.cascade_shadow_split_linear_log_blend_ratio,
          )
          .map(ViewerShadowPreparer::Cascade)
        })
      } else {
        cx.scope(|cx| {
          let source_world = use_global_node_world_mat(cx)
            .fanout(cx.use_db_rev_ref_tri_view::<DirectionalRefNode>(), cx);

          let source_proj = cx
            .use_dual_query::<DirectionLightShadowBound>()
            .dual_query_map(move |orth| {
              orth
                .unwrap_or(DEFAULT_DIR_PROJ)
                .compute_projection_mat(&ndc)
            });

          let size = cx
            .use_dual_query::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>()
            .into_delta_change()
            .map_changes(|size| Size::from_u32_pair_min_one(size.into()));

          let bias = cx
            .use_changes::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>()
            .map_changes(|v| v.into());

          let enabled = cx.use_changes::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>();

          use_basic_shadow_map_uniform(
            cx,
            source_world,
            source_proj,
            size,
            bias,
            enabled,
            *init_config,
          )
          .map(ViewerShadowPreparer::Basic)
        })
      };
      Some(shadow)
    })
  } else {
    None
  };

  let light = use_directional_uniform_array(cx);

  cx.when_render(|| SceneDirectionalLightingPreparer {
    shadow: shadow.map(|s| s.unwrap()),
    light,
  })
}

enum ViewerShadowPreparer {
  Basic((BasicShadowMapPreparer, UniformArray<BasicShadowMapInfo, 8>)),
  Cascade(MultiCascadeShadowMapPreparer),
}

pub struct SceneDirectionalLightingPreparer {
  shadow: Option<ViewerShadowPreparer>,
  light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
}

impl SceneDirectionalLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut impl FnMut(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> Box<dyn LightSystemSceneProvider> {
    if let Some(shadow) = self.shadow {
      match shadow {
        ViewerShadowPreparer::Basic((shadow, info)) => {
          let shadow_map_atlas = shadow.update_shadow_maps(frame_ctx, draw, reversed_depth);

          let provider = SceneDirectionalLightingProvider {
            light: self.light,
            shadow: Some(BasicShadowMapComponent {
              shadow_map_atlas,
              info,
              reversed_depth,
            }),
          };
          Box::new(provider)
        }
        ViewerShadowPreparer::Cascade(cascade_shadow_map_preparer) => {
          let shadow = cascade_shadow_map_preparer.update(frame_ctx, draw, reversed_depth);
          let provider = SceneDirectionalLightingCascadeShadowProvider {
            shadow: Some(shadow),
            light: self.light,
          };
          Box::new(provider)
        }
      }
    } else {
      Box::new(SceneDirectionalLightingCascadeShadowProvider {
        shadow: None,
        light: self.light,
      })
    }
  }
}

pub struct SceneDirectionalLightingProvider {
  light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
  shadow: Option<BasicShadowMapComponent>,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    if let Some(shadow) = &self.shadow {
      let light_with_shadow =
        AbstractShaderBindingIterSourceZip(self.light.clone(), shadow.clone());

      let lights_iter_compute = AbstractShaderBindingIterSourceMap(
        light_with_shadow,
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

      let com = ArrayLights(lights_iter_compute);
      Some(Box::new(com))
    } else {
      Some(dir_light_no_shadow(&self.light))
    }
  }
}

pub fn dir_light_no_shadow(
  lights: &UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
) -> Box<dyn LightingComputeComponent> {
  let lights_iter_compute = AbstractShaderBindingIterSourceMap(
    lights.clone(),
    |(_, light): (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightUniform>)| {
      let light_uniform = light.load().expand();
      ENode::<DirectionalShaderInfo> {
        illuminance: light_uniform.illuminance,
        direction: light_uniform.direction,
      }
      .construct()
    },
  );

  let com = ArrayLights(lights_iter_compute);
  Box::new(com)
}

pub fn use_scene_point_light_uniform(
  cx: &mut QueryGPUHookCx,
) -> Option<ScenePointLightingProvider> {
  let uniform = use_point_uniform_array(cx);
  cx.when_render(|| ScenePointLightingProvider { uniform })
}

pub struct ScenePointLightingProvider {
  uniform: UniformBufferDataView<Shader140Array<PointLightUniform, 8>>,
}

impl LightSystemSceneProvider for ScenePointLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights_iter_compute = AbstractShaderBindingIterSourceMap(
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

    let com = ArrayLights(lights_iter_compute);
    Some(Box::new(com))
  }
}

pub fn use_scene_spot_light_uniform(
  cx: &mut QueryGPUHookCx,
  init_config: &MultiLayerTexturePackerConfig,
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
) -> Option<SceneSpotLightingPreparer> {
  let source_world =
    use_global_node_world_mat(cx).fanout(cx.use_db_rev_ref_tri_view::<SpotLightRefNode>(), cx);

  let source_proj = cx
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

  let size = cx
    .use_dual_query::<BasicShadowMapResolutionOf<SpotLightBasicShadowInfo>>()
    .into_delta_change()
    .map_changes(|size| Size::from_u32_pair_min_one(size.into()));

  let bias = cx
    .use_changes::<BasicShadowMapBiasOf<SpotLightBasicShadowInfo>>()
    .map_changes(|v| v.into());

  let enabled = cx.use_changes::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>();

  let shadow = if lighting_sys.enable_shadow {
    cx.scope(|cx| {
      Some(use_basic_shadow_map_uniform(
        cx,
        source_world,
        source_proj,
        size,
        bias,
        enabled,
        *init_config,
      ))
    })
  } else {
    None
  };

  let light = use_spot_uniform_array(cx);

  cx.when_render(|| {
    let shadow = shadow.map(|s| s.unwrap());
    SceneSpotLightingPreparer { shadow, light }
  })
}

pub struct SceneSpotLightingPreparer {
  pub shadow: Option<(
    BasicShadowMapPreparer,
    UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
  )>,
  pub light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
}

impl SceneSpotLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut impl FnMut(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> SceneSpotLightingProvider {
    let shadow = if let Some(shadow) = self.shadow {
      let shadow_map_atlas = shadow.0.update_shadow_maps(frame_ctx, draw, reversed_depth);

      BasicShadowMapComponent {
        shadow_map_atlas,
        info: shadow.1,
        reversed_depth,
      }
      .into()
    } else {
      None
    };

    SceneSpotLightingProvider {
      light: self.light,
      shadow,
    }
  }
}

pub struct SceneSpotLightingProvider {
  light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
  shadow: Option<BasicShadowMapComponent>,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    if let Some(shadow) = &self.shadow {
      let light_with_shadow =
        AbstractShaderBindingIterSourceZip(self.light.clone(), shadow.clone());

      let lights_iter_compute = AbstractShaderBindingIterSourceMap(
        light_with_shadow,
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

      let com = ArrayLights(lights_iter_compute);
      Some(Box::new(com))
    } else {
      let lights_iter_compute = AbstractShaderBindingIterSourceMap(
        self.light.clone(),
        |(_, light): (Node<u32>, ShaderReadonlyPtrOf<SpotLightUniform>)| {
          let light_uniform = light.load().expand();
          ENode::<SpotLightShaderInfo> {
            luminance_intensity: light_uniform.luminance_intensity,
            position: hpt_uniform_to_hpt(light_uniform.position),
            direction: light_uniform.direction,
            cutoff_distance: light_uniform.cutoff_distance,
            half_cone_cos: light_uniform.half_cone_cos,
            half_penumbra_cos: light_uniform.half_penumbra_cos,
          }
          .construct()
        },
      );

      let com = ArrayLights(lights_iter_compute);
      Some(Box::new(com))
    }
  }
}
