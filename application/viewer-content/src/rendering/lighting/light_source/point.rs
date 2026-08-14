use rendiation_lighting_punctual::PointLightShaderInfo;
use rendiation_lighting_punctual::PunctualShaderLight;

use crate::*;

pub fn use_scene_point_light_uniform(
  cx: &mut QueryGPUHookCx,
  shadow_packer_config: &MultiLayerTexturePackerConfig,
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
) -> Option<ScenePointLightingPreparer> {
  cx.next_scope_index();
  let point_light_uniforms = use_point_per_scene_uniform_array_buffers(cx);

  let shadow = if lighting_sys.enable_shadow {
    cx.scope(|cx| {
      let shadow_info =
        use_cube_shadow_map_uniform(cx, shadow_packer_config, ndc, &point_light_uniforms);
      use_shadow_map_entry(cx, lighting_sys, ndc, shadow_info)
    })
  } else {
    None
  };

  point_light_uniforms.map(|light| ScenePointLightingPreparer {
    shadow,
    light,
    scene_ref: read_global_db_foreign_key(),
    bias_behavior: lighting_sys.bias_behavior,
  })
}

fn use_cube_shadow_map_uniform(
  cx: &mut QueryGPUHookCx,
  atlas_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
  lights: &Option<SharedLightUniformInfo<PointLightUniform>>,
) -> Option<(CubeShadowMapPreparer, SizeWithDepth)> {
  // let changed = cx.use_db_entity_any_change::<PointLightEntity>(); // todo reactive
  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  let gpu = cx.gpu;
  let (cx, gpu_data) = cx.use_plain_state_default::<Option<CubeShadowMapInfoGPU>>();

  cx.when_render(|| {
    let light_ref_node = get_db_view::<PointLightRefNode>();

    let shadow_enabled = get_db_view::<BasicShadowMapEnabledOf<PointLightBasicShadowInfo>>();
    let shadow_map_size = get_db_view::<BasicShadowMapResolutionOf<PointLightBasicShadowInfo>>();
    let shadow_bias = get_db_view::<BasicShadowMapBiasOf<PointLightBasicShadowInfo>>();
    let cutoff_distance = get_db_view::<PointLightCutOffDistance>();
    let world_mat = world_mat.expect_resolve_stage();

    let shadow_info_access = |light_id: RawEntityHandle| {
      let enabled = shadow_enabled.access(&light_id).unwrap();
      if !enabled {
        return None;
      }
      let node = light_ref_node.access(&light_id).unwrap().unwrap();
      let light_world = world_mat.access(&node).unwrap();
      let size = shadow_map_size.access(&light_id).unwrap();
      let bias = shadow_bias.access(&light_id).unwrap();

      let projection = PerspectiveProjection {
        near: 0.1,
        far: cutoff_distance.access(&light_id).unwrap(),
        fov: Deg::from_rad(std::f32::consts::FRAC_PI_2),
        aspect: 1.,
      };
      let proj = ShadowCameraProjectionMatrixes {
        render_matrix: projection.compute_projection_mat(&ndc),
        opengl_ndc_matrix: projection.compute_projection_mat(&OpenGLxNDC),
      };

      CubeShadowMapInfoInput {
        light_world,
        proj,
        map_size: Size::from_u32_pair_min_one(size.into()),
        bias: bias.into(),
      }
      .into()
    };

    let lights = lights.as_ref().unwrap().read();
    prepare_cube_shadow_map_uniform(
      atlas_config,
      &lights.allocation_info,
      &shadow_info_access,
      gpu_data,
      gpu,
    )
  })
}

pub struct ScenePointLightingPreparer {
  pub shadow: Option<ShadowMapPreparerEntry<CubeShadowMapPreparer>>,
  pub light: SharedLightUniformInfo<PointLightUniform>,
  pub scene_ref: ForeignKeyReadView<PointLightRefScene>,
  pub bias_behavior: ShadowBiasBehaviorConfig,
}

impl ScenePointLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest, EntityHandle<SceneEntity>),
    reversed_depth: bool,
  ) -> ScenePointLightingProvider {
    let mut draw = |f_ctx: &mut FrameCtx<'_>, param: ShadowMapDrawRequest| {
      let light_id = unsafe { EntityHandle::from_raw(param.light_id) };
      let scene_id = self
        .scene_ref
        .get(light_id)
        .expect("lighting missing scene ref");

      draw(f_ctx, param, scene_id);
    };

    let shadow = self.shadow.map(|mut entry| {
      let shadow_info =
        entry
          .preparer
          .update_shadow_maps(frame_ctx, entry.shadow_map.as_mut(), &mut draw);
      ShadowMapGPUDataEntry {
        gpu_data: shadow_info,
        shadow_map: entry.shadow_map,
      }
    });

    ScenePointLightingProvider {
      uniform: self.light.make_read_holder(),
      shadow,
      reversed_depth,
      bias_behavior: self.bias_behavior,
    }
  }
}

pub struct ScenePointLightingProvider {
  shadow: Option<ShadowMapGPUDataEntry<CubeShadowMapInfoGPU>>,
  uniform: LockReadGuardHolder<LightUniformInfo<PointLightUniform>>,
  reversed_depth: bool,
  bias_behavior: ShadowBiasBehaviorConfig,
}

impl LightSystemSceneProvider for ScenePointLightingProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights = self.uniform.uniform.get(scene.raw_handle_ref())?.clone();

    let shadow = self.shadow.as_ref().map(|entry| {
      let info = entry
        .gpu_data
        .uniforms
        .get(scene.raw_handle_ref())
        .unwrap()
        .clone();
      CubeShadowMapComponent {
        shadow_computer: entry.shadow_map.create_abstract_shadow_computer(),
        info,
        reversed_depth: self.reversed_depth,
        bias_behavior: self.bias_behavior,
      }
    });

    Some(Box::new(PointLightShader { lights, shadow }))
  }
}

type UniformArray = UniformArrayWithLengthInfo<PointLightUniform, LIGHT_LIST_LEN>;

#[derive(Clone)]
struct PointLightShader {
  lights: UniformBufferCachedDataView<UniformArray>,
  shadow: Option<CubeShadowMapComponent>,
}

impl ShaderHashProvider for PointLightShader {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.shadow.is_some());
    if let Some(shadow) = &self.shadow {
      shadow.hash_pipeline(hasher);
    }
  }
}

impl LightingComputeComponent for PointLightShader {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(PointLightInvocation {
      lights: binding.bind_by(&self.lights),
      shadow: self.shadow.as_ref().map(|s| s.bind_shader(binding)),
    })
  }

  fn setup_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.lights);
    if let Some(s) = &self.shadow {
      s.bind_pass(ctx);
    }
  }
}

struct PointLightInvocation {
  lights: ShaderReadonlyPtrOf<UniformArray>,
  shadow: Option<CubeShadowMapInvocation>,
}

impl LightingComputeInvocation for PointLightInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    light_iter_sum(self.lights.clone().into_shader_iter().map(
      |(shadow_idx, light_ptr): (Node<u32>, ShaderReadonlyPtrOf<PointLightUniform>)| {
        let uniform = light_ptr.load().expand();
        let light = ENode::<PointLightShaderInfo> {
          luminance_intensity: uniform.luminance_intensity,
          position: hpt_uniform_to_hpt(uniform.position),
          cutoff_distance: uniform.cutoff_distance,
        }
        .construct();
        let incident = light.compute_incident_light(geom_ctx);

        let occlusion = match &self.shadow {
          Some(s) => s.query_shadow_occlusion_by_idx(
            geom_ctx.position,
            geom_ctx.normal,
            shadow_idx,
            geom_ctx.fragment_position.xy(),
            geom_ctx.camera_world_position,
          ),
          None => val(1.0),
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
