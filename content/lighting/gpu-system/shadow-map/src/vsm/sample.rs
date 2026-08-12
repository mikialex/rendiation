use super::*;
use crate::*;

/// the vsm moments are sampled with the linear filter so the moments are
/// interpolated across texels, the clamp address mode keeps the sampling
/// inside the atlas
pub fn create_vsm_sampler_desc() -> SamplerDescriptor<'static> {
  SamplerDescriptor {
    mag_filter: rendiation_webgpu::FilterMode::Linear,
    min_filter: rendiation_webgpu::FilterMode::Linear,
    mipmap_filter: rendiation_webgpu::MipmapFilterMode::Nearest,
    address_mode_u: rendiation_webgpu::AddressMode::ClampToEdge,
    address_mode_v: rendiation_webgpu::AddressMode::ClampToEdge,
    ..Default::default()
  }
}

/// maps the value into [0, 1] between a and b
#[shader_fn]
pub fn vsm_linstep(a: Node<f32>, b: Node<f32>, v: Node<f32>) -> Node<f32> {
  ((v - a) / (b - a)).saturate()
}

/// reduces the light bleeding by removing the tail below the amount and
/// rescaling the remaining range
#[shader_fn]
pub fn vsm_reduce_light_bleeding(p_max: Node<f32>, amount: Node<f32>) -> Node<f32> {
  vsm_linstep_fn(amount, val(1.), p_max)
}

/// the one-tailed Chebyshev upper bound of the shadowing probability,
/// aligned with ChebyshevUpperBound in the MJP Shadows sample, the moments
/// are the linear depth moments (E[x], E[x * x]) and the min_variance is
/// the bias that keeps the variance away from zero
#[shader_fn]
pub fn vsm_chebyshev_upper_bound(
  moments: Node<Vec2<f32>>,
  mean: Node<f32>,
  min_variance: Node<f32>,
  light_bleeding_reduction: Node<f32>,
) -> Node<f32> {
  let variance = (moments.y() - moments.x() * moments.x()).max(min_variance);
  let d = mean - moments.x();
  let p_max = variance / (variance + d * d);
  let p_max = vsm_reduce_light_bleeding_fn(p_max, light_bleeding_reduction);
  mean.less_equal_than(moments.x()).select(val(1.), p_max)
}

pub struct VSMComputer {
  pub vsm_map_atlas: GPU2DArrayTextureView,
  pub config: UniformBufferDataView<VSMConfigUniform>,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for VSMComputer {
  shader_hash_type_id! {}
}

impl AbstractShaderBindingSource for VSMComputer {
  type ShaderBindResult = Box<dyn AbstractShadowComputerInvocation>;

  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    Box::new(VSMComputerInvocation {
      vsm_map_atlas: cx.bind_by(&self.vsm_map_atlas),
      sampler: cx.bind_by(&ImmediateGPUSamplerViewBind),
      config: cx.bind_by(&self.config).load().expand(),
      reversed_depth: self.reversed_depth,
    })
  }
}

impl AbstractBindingSource for VSMComputer {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.vsm_map_atlas);
    ctx.bind_immediate_sampler(&create_vsm_sampler_desc());
    ctx.bind(&self.config);
  }
}

struct VSMComputerInvocation {
  vsm_map_atlas: BindingNode<ShaderTexture2DArray>,
  sampler: BindingNode<ShaderSampler>,
  config: ENode<VSMConfigUniform>,
  reversed_depth: bool,
}

impl AbstractShadowComputerInvocation for VSMComputerInvocation {
  fn compute_shadow(
    &self,
    shadow_position: Node<Vec3<f32>>,
    _screen_position: Node<Vec2<f32>>,
    map_info: Node<ShadowMapAddressInfo>,
    _cascade_scale: Node<f32>,
    proj_linear_depth_recover_helper: ShaderReadonlyPtrOf<ProjLinearDepthRecoverHelper>,
  ) -> Node<f32> {
    sample_shadow_map_vsm_fn(
      self.vsm_map_atlas,
      self.sampler,
      shadow_position,
      map_info,
      proj_linear_depth_recover_helper.load(),
      val(self.reversed_depth),
      self.config.vsm_bias,
      self.config.light_bleeding_reduction,
    )
  }
}

/// samples the vsm moments and evaluates the shadowing with the one-tailed
/// Chebyshev upper bound, the compared depth is recovered into the linear
/// depth space that the moments are stored in, see recover_linear_depth,
/// the vsm_bias is scaled by 0.01 to get the min variance like in the MJP
/// Shadows sample
#[shader_fn]
pub fn sample_shadow_map_vsm(
  map: BindingNode<ShaderTexture2DArray>,
  sampler: BindingNode<ShaderSampler>,
  shadow_position: Node<Vec3<f32>>,
  info: Node<ShadowMapAddressInfo>,
  proj_linear_depth_recover_helper: Node<ProjLinearDepthRecoverHelper>,
  reversed_depth: Node<bool>,
  vsm_bias: Node<f32>,
  light_bleeding_reduction: Node<f32>,
) -> Node<f32> {
  let map_size = map.texture_dimension_2d(None).into_f32();
  let uv = map_uv_to_atlas_uv_fn(shadow_position.xy(), info, map_size);
  let depth = recover_linear_depth_fn(
    shadow_position.z(),
    proj_linear_depth_recover_helper,
    reversed_depth,
  );

  let moments = map
    .build_sample_call(sampler, uv)
    .with_array_index(info.expand().layer_index)
    .sample()
    .xy();

  vsm_chebyshev_upper_bound_fn(
    moments,
    depth,
    vsm_bias * val(0.01),
    light_bleeding_reduction,
  )
}
