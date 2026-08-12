use crate::*;

mod fixed_size;
mod grid;
mod optimized;
mod random_disc;
pub use fixed_size::*;
pub use grid::*;
pub use optimized::*;
pub use random_disc::*;

pub struct PCFComputer {
  pub shadow_map_atlas: GPU2DArrayDepthTextureView,
  pub pcf_config_parameter: UniformBufferDataView<PCFConfigParameter>,
  pub pcf_config: ShadowPCFConfig,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for PCFComputer {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(&self.reversed_depth);
    hasher.hash(&self.pcf_config);
  }
}

impl AbstractShaderBindingSource for PCFComputer {
  type ShaderBindResult = Box<dyn AbstractShadowComputerInvocation>;

  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    Box::new(PCFComputerInvocation {
      shadow_map_atlas: cx.bind_by(&self.shadow_map_atlas),
      sampler: cx.bind_by(&ImmediateGPUCompareSamplerViewBind),
      pcf_config_parameter: cx.bind_by(&self.pcf_config_parameter).load().expand(),
      pcf_config: self.pcf_config,
      reversed_depth: self.reversed_depth,
    })
  }
}

impl AbstractBindingSource for PCFComputer {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.shadow_map_atlas);
    ctx.bind_immediate_sampler(&create_shadow_depth_sampler_desc(self.reversed_depth));
    ctx.bind(&self.pcf_config_parameter);
  }
}

struct PCFComputerInvocation {
  shadow_map_atlas: BindingNode<ShaderDepthTexture2DArray>,
  sampler: BindingNode<ShaderCompareSampler>,
  pcf_config_parameter: ENode<PCFConfigParameter>,
  pcf_config: ShadowPCFConfig,
  reversed_depth: bool,
}

impl AbstractShadowComputerInvocation for PCFComputerInvocation {
  fn compute_shadow(
    &self,
    shadow_position: Node<Vec3<f32>>,
    screen_position: Node<Vec2<f32>>,
    map_info: Node<ShadowMapAddressInfo>,
    cascade_scale: Node<f32>,
    _proj_linear_depth_recover_helper: ShaderReadonlyPtrOf<ProjLinearDepthRecoverHelper>,
  ) -> Node<f32> {
    self.pcf_config.sample_shadow_pcf(
      self.shadow_map_atlas,
      self.sampler,
      shadow_position,
      screen_position,
      map_info,
      self.pcf_config_parameter.pcf_filter_size * cascade_scale,
      self.pcf_config_parameter.pcf_num_disc_samples,
      val(self.reversed_depth),
    )
  }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ShadowPCFMode {
  /// fixed size PCF optimized with GatherCmp, from "Fast Conventional Shadow Filtering"
  FixedSizePCF,
  /// the method used in The Witness, decompose the filter kernel into bilinear weighted samples
  OptimizedPCF,
  /// grid sampled PCF with dynamic filter size and edge coverage weights
  GridPCF,
  /// PCF with a kernel made up from random points on a disc
  RandomDiscPCF,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct PCFConfigParameter {
  /// the PCF filter size in texels
  pub pcf_filter_size: f32,
  /// the sample count of the random disc PCF
  pub pcf_num_disc_samples: u32,
}

pub fn create_pcf_parameter(
  gpu: &GPU,
  pcf_config: ShadowPCFConfig,
) -> UniformBufferDataView<PCFConfigParameter> {
  // todo cache
  let pcf_parameter = create_uniform(
    PCFConfigParameter {
      pcf_filter_size: pcf_config.filter_size,
      pcf_num_disc_samples: pcf_config
        .num_disc_samples
        .min(POISSON_SAMPLES.len() as u32),
      ..Default::default()
    },
    &gpu.device,
    "pcf_parameter",
  );

  pcf_parameter
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowPCFConfig {
  pub pcf_mode: ShadowPCFMode,
  /// the filter kernel size of the fixed size PCF
  pub fixed_filter_size: FixedFilterSize,
  /// the filter size in texels for GridPCF and RandomDiscPCF,
  /// passed as uniform so it can be adjusted at runtime without recompiling the shader
  pub filter_size: f32,
  /// the sample count for RandomDiscPCF, passed as uniform
  pub num_disc_samples: u32,
  /// use receiver plane depth bias
  pub use_receiver_plane_depth_bias: bool,
}

impl std::hash::Hash for ShadowPCFConfig {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.pcf_mode.hash(state);
    self.fixed_filter_size.hash(state);
    self.use_receiver_plane_depth_bias.hash(state);
  }
}

impl Default for ShadowPCFConfig {
  fn default() -> Self {
    Self {
      pcf_mode: ShadowPCFMode::OptimizedPCF,
      fixed_filter_size: FixedFilterSize::Filter3x3,
      filter_size: 3.0,
      num_disc_samples: 32,
      use_receiver_plane_depth_bias: false,
    }
  }
}

impl ShadowPCFConfig {
  pub fn sample_shadow_pcf(
    self,
    map: BindingNode<ShaderDepthTexture2DArray>,
    sampler: BindingNode<ShaderCompareSampler>,
    shadow_position: Node<Vec3<f32>>,
    random_seed: Node<Vec2<f32>>,
    map_info: Node<ShadowMapAddressInfo>,
    filter_size: Node<f32>,
    num_disc_samples: Node<u32>,
    reversed_depth: Node<bool>,
  ) -> Node<f32> {
    // the receiver plane depth bias is derived from the shadow map uv
    // derivatives, which are per-cascade, so it has to be recomputed
    // for the next cascade instead of reusing the current one
    let receiver_plane_depth_bias = if self.use_receiver_plane_depth_bias {
      let shadow_pos_dx = shadow_position.dpdx_fine();
      let shadow_pos_dy = shadow_position.dpdy_fine();
      compute_receiver_plane_depth_bias_fn(shadow_pos_dx, shadow_pos_dy)
    } else {
      val(Vec2::zero())
    };

    match self.pcf_mode {
      ShadowPCFMode::FixedSizePCF => sample_shadow_pcf_fixed_size(
        map,
        sampler,
        shadow_position,
        map_info,
        receiver_plane_depth_bias,
        self.fixed_filter_size,
        reversed_depth,
      ),
      ShadowPCFMode::OptimizedPCF => sample_shadow_pcf_optimized_fn(
        map,
        sampler,
        shadow_position,
        map_info,
        receiver_plane_depth_bias,
        reversed_depth,
      ),
      ShadowPCFMode::GridPCF => sample_shadow_pcf_grid_fn(
        map,
        sampler,
        shadow_position,
        map_info,
        filter_size.splat(),
        receiver_plane_depth_bias,
        reversed_depth,
      ),
      ShadowPCFMode::RandomDiscPCF => sample_shadow_pcf_random_disc_fn(
        map,
        sampler,
        shadow_position,
        random_seed,
        map_info,
        filter_size.splat(),
        num_disc_samples,
        receiver_plane_depth_bias,
        reversed_depth,
      ),
    }
  }
}

/// convert the shadow map uv to the atlas uv
#[shader_fn]
pub fn map_uv_to_atlas_uv(
  uv: Node<Vec2<f32>>,
  info: Node<ShadowMapAddressInfo>,
  atlas_size: Node<Vec2<f32>>,
) -> Node<Vec2<f32>> {
  let info = info.expand();
  uv * (info.size / atlas_size) + info.offset / atlas_size
}

/// the static depth bias to make up for incorrect fractional sampling on the shadow map grid
#[shader_fn]
pub fn fractional_sampling_error(
  info: Node<ShadowMapAddressInfo>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
) -> Node<f32> {
  let info = info.expand();
  let texel_size = val(Vec2::new(1., 1.)) / info.size;
  (val(Vec2::new(1., 1.)) * texel_size).dot(receiver_plane_depth_bias.abs())
}

/// apply the static depth biasing that makes up for incorrect fractional sampling
/// on the shadow map grid, signed by the depth space like the depth bias itself
#[shader_fn]
fn apply_fractional_sampling_error(
  light_depth: Node<f32>,
  fractional_sampling_error: Node<f32>,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let error = fractional_sampling_error.min(val(0.01));
  reversed_depth.select(light_depth + error, light_depth - error)
}

/// Calculates the offset to use for sampling the shadow map, based on the surface normal
/// aligns with ComputeReceiverPlaneDepthBias in the MJP Shadows sample
#[shader_fn]
pub fn compute_receiver_plane_depth_bias(
  tex_coord_dx: Node<Vec3<f32>>,
  tex_coord_dy: Node<Vec3<f32>>,
) -> Node<Vec2<f32>> {
  let bias_u = tex_coord_dy.y() * tex_coord_dx.z() - tex_coord_dx.y() * tex_coord_dy.z();
  let bias_v = tex_coord_dx.x() * tex_coord_dy.z() - tex_coord_dy.x() * tex_coord_dx.z();
  let scale = val(1.) / (tex_coord_dx.x() * tex_coord_dy.y() - tex_coord_dx.y() * tex_coord_dy.x());
  (bias_u * scale, bias_v * scale).into()
}
