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

pub fn area_light_uniform_array(gpu: &GPU) -> UniformArrayUpdateContainer<LTCAreaLightUniform, 8> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let intensity = global_watch()
    .watch::<AreaLightIntensity>()
    .into_query_update_uniform_array(offset_of!(LTCAreaLightUniform, intensity), gpu);

  let double_side = global_watch()
    .watch::<AreaLightIsDoubleSide>()
    .collective_map(Bool::from)
    .into_query_update_uniform_array(offset_of!(LTCAreaLightUniform, double_side), gpu);

  let is_disk = global_watch()
    .watch::<AreaLightIsRound>()
    .collective_map(Bool::from)
    .into_query_update_uniform_array(offset_of!(LTCAreaLightUniform, is_disk), gpu);

  let points = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<AreaLightRefNode>())
    .collective_zip(global_watch().watch::<AreaLightSize>())
    .collective_map(|(world_mat, size)| {
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
    .into_query_update_uniform_array(offset_of!(LTCAreaLightUniform, p1), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(points)
    .with_source(intensity)
    .with_source(double_side)
    .with_source(is_disk)
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

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniforms);
    ctx.binding.bind(&self.ltc_1);
    ctx.binding.bind(&self.ltc_2);
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
