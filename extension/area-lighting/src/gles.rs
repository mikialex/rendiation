use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LTCAreaLightUniform {
  /// pre calculated vertex in world space.
  pub p1: HighPrecisionTranslationUniform,
  pub p2: HighPrecisionTranslationUniform,
  pub p3: HighPrecisionTranslationUniform,
  pub p4: HighPrecisionTranslationUniform,
  pub intensity: Vec3<f32>,
  pub double_side: Bool,
  pub is_disk: Bool,
}

pub fn use_area_light_uniform_array(
  cx: &mut QueryGPUHookCx,
) -> UniformArray<LTCAreaLightUniform, 8> {
  let (cx, uniform) = cx.use_uniform_array_buffers();

  let offset = offset_of!(LTCAreaLightUniform, intensity);
  cx.use_changes::<AreaLightIntensity>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(LTCAreaLightUniform, double_side);
  cx.use_changes::<AreaLightIsDoubleSide>()
    .map_changes(Bool::from)
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(LTCAreaLightUniform, is_disk);
  cx.use_changes::<AreaLightIsRound>()
    .map_changes(Bool::from)
    .update_uniform_array(uniform, offset, cx.gpu);

  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<AreaLightRefNode>(), cx)
    .dual_query_zip(cx.use_dual_query::<AreaLightSize>())
    .dual_query_map(|(world_mat, size)| {
      let width = size.x as f64 / 2.;
      let height = size.y as f64 / 2.;
      let p1 = world_mat * Vec3::new(width, height, 0.);
      let p2 = world_mat * Vec3::new(-width, height, 0.);
      let p3 = world_mat * Vec3::new(-width, -height, 0.);
      let p4 = world_mat * Vec3::new(width, -height, 0.);
      [
        into_hpt(p1).into_uniform(),
        into_hpt(p2).into_uniform(),
        into_hpt(p3).into_uniform(),
        into_hpt(p4).into_uniform(),
      ]
    })
    .use_assure_result(cx)
    .into_delta_change()
    .update_uniform_array(uniform, offset_of!(LTCAreaLightUniform, p1), cx.gpu);

  uniform.clone()
}

pub struct SceneAreaLightingProvider {
  pub ltc_1: GPU2DTextureView,
  pub ltc_2: GPU2DTextureView,
  pub uniform: UniformBufferDataView<Shader140Array<LTCAreaLightUniform, 8>>,
}

impl LightSystemSceneProvider for SceneAreaLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _camera: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    Some(Box::new(LTCLightingComputeComponent {
      ltc_1: self.ltc_1.clone(),
      ltc_2: self.ltc_2.clone(),
      uniforms: self.uniform.clone(),
    }))
  }
}

pub struct LTCLightingComputeComponent {
  ltc_1: GPU2DTextureView,
  ltc_2: GPU2DTextureView,
  uniforms: UniformBufferDataView<Shader140Array<LTCAreaLightUniform, 8>>,
}
impl ShaderHashProvider for LTCLightingComputeComponent {
  shader_hash_type_id! {}
}

impl LightingComputeComponent for LTCLightingComputeComponent {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    _scene_id: Node<u32>, // todo
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
  uniforms: ShaderReadonlyPtrOf<Shader140Array<LTCAreaLightUniform, 8>>,
  lut: LTCxLUTxInvocation,
}

impl LightingComputeInvocation for LTCLightingComputeInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let lut = self.lut;
    let lights = self.uniforms.clone().map(
      move |(_, u): (Node<u32>, ShaderReadonlyPtrOf<LTCAreaLightUniform>)| {
        let u = u.load().expand();
        LTCRectLightingCompute {
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
      },
    );

    ShaderIntoIterAsLightInvocation(lights).compute_lights(shading, geom_ctx)
  }
}
