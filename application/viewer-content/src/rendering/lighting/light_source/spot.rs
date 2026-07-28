use rendiation_lighting_punctual::PunctualShaderLight;
use rendiation_lighting_punctual::SpotLightShaderInfo;

use crate::*;

pub fn use_scene_spot_light_uniform(
  cx: &mut QueryGPUHookCx,
  shadow_packer_config: &MultiLayerTexturePackerConfig,
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
) -> Option<SceneSpotLightingPreparer> {
  cx.next_scope_index();
  let spot_light_uniforms = use_spot_per_scene_uniform_array_buffers(cx);

  let shadow = if lighting_sys.enable_shadow {
    cx.scope(|cx| use_basic_shadow_map_uniform(cx, shadow_packer_config, ndc, &spot_light_uniforms))
  } else {
    None
  };

  spot_light_uniforms.map(|light| SceneSpotLightingPreparer {
    shadow,
    light,
    scene_ref: read_global_db_foreign_key(),
  })
}

fn use_basic_shadow_map_uniform(
  cx: &mut QueryGPUHookCx,
  atlas_config: &MultiLayerTexturePackerConfig,
  ndc: ViewerNDC,
  lights: &Option<SharedLightUniformInfo<SpotLightUniform>>,
) -> Option<BasicShadowMapPreparer> {
  // // let changed = cx.use_db_entity_any_change::<DirectionalLightEntity>(); // todo
  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  let gpu = cx.gpu;
  let (cx, gpu_data) = cx.use_plain_state_default::<Option<BasicShadowMapGPU>>();

  cx.when_render(|| {
    let light_ref_node = get_db_view::<SpotLightRefNode>();

    let shadow_enabled = get_db_view::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>();
    let shadow_map_size = get_db_view::<BasicShadowMapResolutionOf<SpotLightBasicShadowInfo>>();
    let shadow_bias = get_db_view::<BasicShadowMapBiasOf<SpotLightBasicShadowInfo>>();
    let world_mat = world_mat.expect_resolve_stage();
    let half_cone = get_db_view::<SpotLightHalfConeAngle>();

    let shadow_info_access = |light_id: RawEntityHandle| {
      let enabled = shadow_enabled.access(&light_id).unwrap();
      if !enabled {
        return None;
      }
      let node = light_ref_node.access(&light_id).unwrap().unwrap();
      let light_world = world_mat.access(&node).unwrap();
      let size = shadow_map_size.access(&light_id).unwrap();
      let bias = shadow_bias.access(&light_id).unwrap();

      let half_cone_angle = half_cone.access(&light_id).unwrap();
      let proj = PerspectiveProjection {
        near: 0.1,
        far: 2000.,
        fov: Deg::from_rad(half_cone_angle * 2.),
        aspect: 1.,
      }
      .compute_projection_mat(&ndc);

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

pub struct SceneSpotLightingPreparer {
  pub shadow: Option<BasicShadowMapPreparer>,
  pub light: SharedLightUniformInfo<SpotLightUniform>,
  pub scene_ref: ForeignKeyReadView<SpotLightRefScene>,
}

impl SceneSpotLightingPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    draw: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest, EntityHandle<SceneEntity>),
    reversed_depth: bool,
  ) -> SceneSpotLightingProvider {
    let mut draw = |f_ctx: &mut FrameCtx<'_>, param: ShadowMapDrawRequest| {
      let light_id = unsafe { EntityHandle::from_raw(param.light_id) };
      let scene_id = self
        .scene_ref
        .get(light_id)
        .expect("lighting missing scene ref");

      draw(f_ctx, param, scene_id);
    };

    let shadow = self
      .shadow
      .map(|v| v.update_shadow_maps(frame_ctx, &mut draw, reversed_depth));

    SceneSpotLightingProvider {
      uniform: self.light.make_read_holder(),
      shadow,
      reversed_depth,
    }
  }
}

pub struct SceneSpotLightingProvider {
  shadow: Option<BasicShadowMapGPU>,
  uniform: LockReadGuardHolder<LightUniformInfo<SpotLightUniform>>,
  reversed_depth: bool,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights = self.uniform.uniform.get(scene.raw_handle_ref())?.clone();

    let shadow = self.shadow.as_ref().map(|s| {
      let info = s.uniforms.get(scene.raw_handle_ref()).unwrap().clone();
      BasicShadowMapComponent {
        shadow_map_atlas: s.shadow_map.get_full_view().clone(),
        info,
        reversed_depth: self.reversed_depth,
      }
    });

    Some(Box::new(SpotLightShader { lights, shadow }))
  }
}

type UniformArray = UniformArrayWithLengthInfo<SpotLightUniform, LIGHT_LIST_LEN>;

#[derive(Clone)]
struct SpotLightShader {
  lights: UniformBufferCachedDataView<UniformArray>,
  shadow: Option<BasicShadowMapComponent>,
}

impl ShaderHashProvider for SpotLightShader {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.shadow.is_some());
  }
}

impl LightingComputeComponent for SpotLightShader {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(SpotLightInvocation {
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

struct SpotLightInvocation {
  lights: ShaderReadonlyPtrOf<UniformArray>,
  shadow: Option<BasicShadowMapInvocation>,
}

impl LightingComputeInvocation for SpotLightInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    light_iter_sum(self.lights.clone().into_shader_iter().map(
      |(shadow_idx, light_ptr): (Node<u32>, ShaderReadonlyPtrOf<SpotLightUniform>)| {
        let uniform = light_ptr.load().expand();
        let light = ENode::<SpotLightShaderInfo> {
          luminance_intensity: uniform.luminance_intensity,
          position: hpt_uniform_to_hpt(uniform.position),
          direction: uniform.direction,
          cutoff_distance: uniform.cutoff_distance,
          half_cone_cos: uniform.half_cone_cos,
          half_penumbra_cos: uniform.half_penumbra_cos,
        }
        .construct();
        let incident = light.compute_incident_light(geom_ctx);

        let occlusion = match &self.shadow {
          Some(s) => s.query_shadow_occlusion_by_idx(
            geom_ctx.position,
            geom_ctx.normal,
            shadow_idx,
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
