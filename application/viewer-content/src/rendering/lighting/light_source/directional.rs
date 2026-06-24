use rendiation_lighting_punctual::DirectionalShaderInfo;
use rendiation_lighting_punctual::PunctualShaderLight;

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
  shadow_packer_config: &MultiLayerTexturePackerConfig,
  viewports: &[ViewerViewPort],
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
) -> Option<SceneDirectionalLightingPreparer> {
  cx.next_scope_index();
  let directional_light_uniforms = use_directional_per_scene_uniform_array_buffers(cx);

  let shadow = if lighting_sys.enable_shadow {
    cx.scope(|cx| {
      if lighting_sys.use_cascade_shadowmap_for_directional_lights {
        cx.scope(|cx| {
          use_cascade_shadow_map(
            cx,
            viewports,
            ndc,
            shadow_packer_config,
            lighting_sys.cascade_shadow_split_linear_log_blend_ratio,
            &directional_light_uniforms,
          )
          .map(ViewerDirectionalShadowPreparer::Cascade)
        })
      } else {
        use_basic_shadow_map_uniform(cx, shadow_packer_config, ndc, &directional_light_uniforms)
          .map(ViewerDirectionalShadowPreparer::Basic)
      }
    })
  } else {
    Some(ViewerDirectionalShadowPreparer::NoShadow)
  };

  directional_light_uniforms.map(|light| SceneDirectionalLightingPreparer {
    shadow: shadow.unwrap(),
    light,
  })
}

fn use_basic_shadow_map_uniform(
  cx: &mut QueryGPUHookCx,
  atlas_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
  lights: &Option<SharedLightUniformInfo<DirectionalLightUniform>>,
) -> Option<BasicShadowMapPreparer> {
  // let changed = cx.use_db_entity_any_change::<DirectionalLightEntity>(); // todo
  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  let gpu = cx.gpu;
  let (cx, gpu_data) = cx.use_plain_state_default::<Option<BasicShadowMapGPU>>();

  cx.when_render(|| {
    let light_ref_node = get_db_view::<DirectionalRefNode>();
    let follow_camera = get_db_view::<DirectionalLightFollowCamera>();

    let shadow_enabled = get_db_view::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>();
    let shadow_map_size =
      get_db_view::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>();
    let shadow_bias = get_db_view::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>();
    let shadow_proj = get_db_view::<DirectionLightShadowBound>();
    let world_mat = world_mat.expect_resolve_stage();

    let shadow_info_access = |light_id: RawEntityHandle| {
      let enabled = shadow_enabled.access(&light_id).unwrap();
      if !enabled {
        return None;
      }
      if follow_camera.access(&light_id).unwrap() && enabled {
        log::warn!("follow camera is not correctly supported when enable shadow");
      }
      let node = light_ref_node.access(&light_id).unwrap().unwrap();
      let light_world = world_mat.access(&node).unwrap();
      let size = shadow_map_size.access(&light_id).unwrap();
      let bias = shadow_bias.access(&light_id).unwrap();
      let orth = shadow_proj
        .access(&light_id)
        .unwrap()
        .unwrap_or(DEFAULT_DIR_PROJ);
      let proj = orth.compute_projection_mat(&ndc);

      BasicShadowMapInfoInput {
        light_world,
        proj,
        map_size: Size::from_u32_pair_min_one(size.into()),
        bias: bias.into(),
      }
      .into()
    };

    let lights = lights.as_ref().unwrap().read();
    prepare_basic_shadow_map_uniform(
      atlas_config,
      &lights.allocation_info,
      &shadow_info_access,
      gpu_data,
      gpu,
    )
  })
}

enum ViewerDirectionalShadowPreparer {
  Basic(BasicShadowMapPreparer),
  Cascade(MultiCascadeShadowMapPreparer),
  NoShadow,
}

pub struct SceneDirectionalLightingPreparer {
  shadow: ViewerDirectionalShadowPreparer,
  light: SharedLightUniformInfo<DirectionalLightUniform>,
}

impl SceneDirectionalLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut impl FnMut(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> Box<dyn LightSystemSceneProvider> {
    let shadows = match self.shadow {
      ViewerDirectionalShadowPreparer::Basic(preparer) => {
        let shadow_gpu_data = preparer.update_shadow_maps(frame_ctx, draw, reversed_depth);
        ShadowImplType::Basic(shadow_gpu_data)
      }
      ViewerDirectionalShadowPreparer::Cascade(cascade_shadow_map_preparer) => {
        let shadow = cascade_shadow_map_preparer.update(frame_ctx, draw, reversed_depth);
        ShadowImplType::Cascade(shadow)
      }
      ViewerDirectionalShadowPreparer::NoShadow => ShadowImplType::NoShadow,
    };

    Box::new(SceneDirectionalLightingProvider {
      lights: self.light.make_read_holder(),
      shadows,
      reversed_depth,
    })
  }
}

enum ShadowImplType {
  NoShadow,
  Basic(BasicShadowMapGPU),
  Cascade(MultiCascadeShadowMapData),
}

enum ShadowImplComType {
  NoShadow,
  Basic(BasicShadowMapComponent),
  Cascade(CascadeShadowMapComponent),
}

enum ShadowImplInvocationType {
  NoShadow,
  Basic(BasicShadowMapInvocation),
  Cascade(CascadeShadowMapInvocation),
}

struct SceneDirectionalLightingProvider {
  lights: LockReadGuardHolder<LightUniformInfo<DirectionalLightUniform>>,
  shadows: ShadowImplType,
  reversed_depth: bool,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights = self.lights.uniform.get(scene.raw_handle_ref())?.clone();

    let shadows = match &self.shadows {
      ShadowImplType::NoShadow => ShadowImplComType::NoShadow,
      ShadowImplType::Basic(s) => {
        let info = s.uniforms.get(scene.raw_handle_ref()).unwrap().clone();
        ShadowImplComType::Basic(BasicShadowMapComponent {
          shadow_map_atlas: s.shadow_map.get_full_view().clone(),
          info,
          reversed_depth: self.reversed_depth,
        })
      }
      ShadowImplType::Cascade(data) => {
        let gpu_data = data.per_camera.get(&camera)?;
        let info = gpu_data.uniforms.get(scene.raw_handle_ref())?.clone();
        ShadowImplComType::Cascade(CascadeShadowMapComponent {
          shadow_map_atlas: gpu_data.shadow_map_atlas.clone(),
          info,
          reversed_depth: self.reversed_depth,
        })
      }
    };

    Some(Box::new(DirectionalLightingShader { lights, shadows }))
  }
}

struct DirectionalLightingShader {
  lights: UniformBufferCachedDataView<UniformArrayWithLengthInfo<DirectionalLightUniform>>,
  shadows: ShadowImplComType,
}

impl LightingComputeComponent for DirectionalLightingShader {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(DirectionalLightingInvocation {
      lights: binding.bind_by(&self.lights),
      shadows: match &self.shadows {
        ShadowImplComType::NoShadow => ShadowImplInvocationType::NoShadow,
        ShadowImplComType::Basic(c) => ShadowImplInvocationType::Basic(c.bind_shader(binding)),
        ShadowImplComType::Cascade(c) => {
          ShadowImplInvocationType::Cascade(AbstractShaderBindingSource::bind_shader(c, binding))
        }
      },
    })
  }

  fn setup_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.lights);
    match &self.shadows {
      ShadowImplComType::NoShadow => {}
      ShadowImplComType::Basic(c) => c.bind_pass(ctx),
      ShadowImplComType::Cascade(c) => AbstractBindingSource::bind_pass(c, ctx),
    }
  }
}

impl ShaderHashProvider for DirectionalLightingShader {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(std::mem::discriminant(&self.shadows));
    match &self.shadows {
      ShadowImplComType::NoShadow => {}
      ShadowImplComType::Basic(_) => {}
      ShadowImplComType::Cascade(c) => c.hash_pipeline(hasher),
    }
  }
}

struct DirectionalLightingInvocation {
  lights: ShaderReadonlyPtrOf<UniformArrayWithLengthInfo<DirectionalLightUniform>>,
  shadows: ShadowImplInvocationType,
}

impl LightingComputeInvocation for DirectionalLightingInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    light_iter_sum(self.lights.clone().into_shader_iter().map(
      |(shadow_idx, light_ptr): (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightUniform>)| {
        let uniform = light_ptr.load().expand();
        let light = ENode::<DirectionalShaderInfo> {
          illuminance: uniform.illuminance,
          direction: uniform.direction,
          follow_camera: uniform.follow_camera,
        }
        .construct();
        let incident = light.compute_incident_light(geom_ctx);

        let occlusion = match &self.shadows {
          ShadowImplInvocationType::NoShadow => val(1.0),
          ShadowImplInvocationType::Basic(s) => s.query_shadow_occlusion_by_idx(
            geom_ctx.position,
            geom_ctx.normal,
            shadow_idx,
            geom_ctx.camera_world_position,
          ),
          ShadowImplInvocationType::Cascade(s) => s.query_shadow_occlusion_by_idx(
            geom_ctx.position,
            geom_ctx.normal,
            shadow_idx,
            geom_ctx.camera_world_position,
            geom_ctx.camera_world_none_translation_mat,
          ),
        };

        shading.compute_lighting_by_incident(
          &ENode::<ShaderIncidentLight> {
            color: incident.color * occlusion,
            direction: incident.direction,
          },
          geom_ctx,
        )
      },
    ))
  }
}
