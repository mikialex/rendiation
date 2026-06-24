use rendiation_scene_rendering_gpu_gles::*;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default, PartialEq)]
pub struct LTCAreaLightUniform {
  /// precalculated vertex in world space.
  pub p1: HighPrecisionTranslationUniform,
  pub p2: HighPrecisionTranslationUniform,
  pub p3: HighPrecisionTranslationUniform,
  pub p4: HighPrecisionTranslationUniform,
  pub intensity: Vec3<f32>,
  pub double_side: Bool,
  pub is_disk: Bool,
}

pub fn use_area_per_scene_uniform_array_buffers(
  cx: &mut QueryGPUHookCx,
) -> Option<SharedLightUniformInfo<LTCAreaLightUniform>> {
  cx.next_scope_index();
  let uniform_array_caches = use_shared_light_uniform_info(cx, "area");

  cx.skip_if_not_waked(|cx| {
    cx.use_db_entity_any_change::<AreaLightEntity>();
    let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

    if cx.is_in_render() {
      let world = world_mat.expect_resolve_stage();
      let r = create_area_light_uniform(&|node| world.access(&node).unwrap());

      sync_per_scene_uniforms(&r, &uniform_array_caches, &cx.gpu, "area");
    }
  });

  cx.when_render(|| uniform_array_caches.clone())
}

pub fn create_area_light_uniform(
  node_world_mat: &dyn Fn(RawEntityHandle) -> Mat4<f64>,
) -> PerSceneLightUniformArray<LTCAreaLightUniform> {
  let light_ref_scene = get_db_view::<AreaLightRefScene>();
  let light_ref_node = get_db_view::<AreaLightRefNode>();

  let intensity = get_db_view::<AreaLightIntensity>();
  let double_side = get_db_view::<AreaLightIsDoubleSide>();
  let is_round = get_db_view::<AreaLightIsRound>();
  let size = get_db_view::<AreaLightSize>();

  let iter_lights = light_ref_scene.iter_key_value().filter_map(|(light, s)| {
    let s = s?;
    let world_mat = node_world_mat(light_ref_node.access(&light)??);
    let size = size.access(&light)?;

    let width = size.x as f64 / 2.;
    let height = size.y as f64 / 2.;
    let p1 = world_mat * Vec3::new(width, height, 0.);
    let p2 = world_mat * Vec3::new(-width, height, 0.);
    let p3 = world_mat * Vec3::new(-width, -height, 0.);
    let p4 = world_mat * Vec3::new(width, -height, 0.);

    let light_data = LTCAreaLightUniform {
      p1: into_hpt(p1).into_uniform(),
      p2: into_hpt(p2).into_uniform(),
      p3: into_hpt(p3).into_uniform(),
      p4: into_hpt(p4).into_uniform(),
      intensity: intensity.access(&light)?,
      double_side: double_side.access(&light)?.into(),
      is_disk: is_round.access(&light)?.into(),
      ..Default::default()
    };

    (light, s, light_data).into()
  });

  compute_light_list(iter_lights)
}

pub struct SceneAreaLightingProvider {
  pub ltc_1: GPU2DTextureView,
  pub ltc_2: GPU2DTextureView,
  pub uniform: LockReadGuardHolder<LightUniformInfo<LTCAreaLightUniform>>,
}

impl LightSystemSceneProvider for SceneAreaLightingProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let lights = self.uniform.uniform.get(scene.raw_handle_ref())?.clone();

    Some(Box::new(LTCLightingComputeComponent {
      ltc_1: self.ltc_1.clone(),
      ltc_2: self.ltc_2.clone(),
      uniforms: lights,
    }))
  }
}

pub struct LTCLightingComputeComponent {
  ltc_1: GPU2DTextureView,
  ltc_2: GPU2DTextureView,
  uniforms: UniformBufferCachedDataView<UniformArrayWithLengthInfo<LTCAreaLightUniform>>,
}
impl ShaderHashProvider for LTCLightingComputeComponent {
  shader_hash_type_id! {}
}

impl LightingComputeComponent for LTCLightingComputeComponent {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(LTCLightingComputeInvocation {
      uniforms: binding.bind_by(&self.uniforms),
      lut: LTCxLUTxInvocation {
        ltc_1: binding.bind_by(&self.ltc_1),
        ltc_2: binding.bind_by(&self.ltc_2),
        sampler: binding.bind_by(&ImmediateGPUSamplerViewBind),
      },
    })
  }

  fn setup_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.uniforms);
    ctx.bind(&self.ltc_1);
    ctx.bind(&self.ltc_2);
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
  }
}

struct LTCLightingComputeInvocation {
  uniforms: ShaderReadonlyPtrOf<UniformArrayWithLengthInfo<LTCAreaLightUniform>>,
  lut: LTCxLUTxInvocation,
}

impl LightingComputeInvocation for LTCLightingComputeInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let specular = val(Vec3::<f32>::splat(0.)).make_local_var();
    let diffuse = val(Vec3::<f32>::splat(0.)).make_local_var();
    let lut = self.lut;

    self
      .uniforms
      .clone()
      .into_shader_iter()
      .for_each(|(_, u), _| {
        let u = u.load().expand();
        let r = LTCRectLightingCompute {
          light: ENode::<LTCRectLight> {
            p1: hpt_uniform_to_hpt(u.p1),
            p2: hpt_uniform_to_hpt(u.p2),
            p3: hpt_uniform_to_hpt(u.p3),
            p4: hpt_uniform_to_hpt(u.p4),
            intensity: u.intensity,
            double_side: u.double_side,
            is_disk: u.is_disk,
          }
          .construct(),
          lut,
        }
        .compute_lights(shading, geom_ctx);

        specular.store(specular.load() + r.specular);
        diffuse.store(diffuse.load() + r.diffuse);
      });

    ENode::<ShaderLightingResult> {
      specular: specular.load(),
      diffuse: diffuse.load(),
    }
  }
}
