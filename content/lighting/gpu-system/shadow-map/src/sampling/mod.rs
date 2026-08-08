use crate::*;

mod fixed_size;
mod grid;
mod optimized;
mod random_disc;
pub use fixed_size::*;
pub use grid::*;
pub use optimized::*;
pub use random_disc::*;

/// the filter kernel size of the [ShadowPCFMode::FixedSizePCF]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum FixedFilterSize {
  Filter3x3,
  Filter5x5,
  Filter7x7,
  Filter9x9,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ShadowPCFMode {
  /// a naive implementation, 3x3 grid sample with linear compare sampler
  Naive,
  /// fixed size PCF optimized with GatherCmp, from "Fast Conventional Shadow Filtering"
  FixedSizePCF,
  /// the method used in The Witness, decompose the filter kernel into bilinear weighted samples
  OptimizedPCF,
  /// grid sampled PCF with dynamic filter size and edge coverage weights
  GridPCF,
  /// PCF with a kernel made up from random points on a disc
  RandomDiscPCF,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowPCFConfig {
  pub pcf_mode: ShadowPCFMode,
  /// the filter kernel size of the fixed size PCF
  pub fixed_filter_size: FixedFilterSize,
  /// use receiver plane depth bias instead of static depth bias
  pub use_receiver_plane_depth_bias: bool,
  /// scale the normal offset by (1 - nDotL), so that the offset is smaller on the lit side
  pub use_n_dot_l_normal_offset: bool,
  /// the filter size in texels for GridPCF and RandomDiscPCF,
  /// passed as uniform so it can be adjusted at runtime without recompiling the shader
  pub filter_size: f32,
  /// the sample count for RandomDiscPCF, passed as uniform
  pub num_disc_samples: u32,
}

impl std::hash::Hash for ShadowPCFConfig {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.pcf_mode.hash(state);
    self.fixed_filter_size.hash(state);
    self.use_receiver_plane_depth_bias.hash(state);
    self.use_n_dot_l_normal_offset.hash(state);
  }
}

impl Default for ShadowPCFConfig {
  fn default() -> Self {
    Self {
      pcf_mode: ShadowPCFMode::Naive,
      fixed_filter_size: FixedFilterSize::Filter3x3,
      use_receiver_plane_depth_bias: false,
      use_n_dot_l_normal_offset: false,
      filter_size: 3.0,
      num_disc_samples: 32,
    }
  }
}

impl ShadowPCFConfig {
  /// compute the depth bias and the receiver plane depth bias for the PCF sampling,
  /// the receiver plane depth bias is only used by the improved modes, the naive
  /// implementation always uses the static depth bias.
  /// the returned depth bias is signed by the depth space: added to the reference
  /// depth in reversed depth space (larger depth is closer) and subtracted in the
  /// standard depth space, so that the reference is always moved away from the light
  pub fn compute_pcf_depth_bias(
    self,
    shadow_position: Node<Vec3<f32>>,
    bias: Node<f32>,
    reversed_depth: bool,
  ) -> (Node<f32>, Node<Vec2<f32>>) {
    if self.use_receiver_plane_depth_bias && self.pcf_mode != ShadowPCFMode::Naive {
      let shadow_pos_dx = shadow_position.dpdx_fine();
      let shadow_pos_dy = shadow_position.dpdy_fine();
      (
        val(0.),
        compute_receiver_plane_depth_bias_fn(shadow_pos_dx, shadow_pos_dy),
      )
    } else {
      let bias = if reversed_depth { bias } else { -bias };
      (bias, val(Vec2::zero()))
    }
  }

  pub fn sample_shadow_pcf(
    self,
    map: BindingNode<ShaderDepthTexture2DArray>,
    sampler: BindingNode<ShaderCompareSampler>,
    shadow_position: Node<Vec3<f32>>,
    random_seed: Node<Vec2<f32>>,
    map_info: Node<ShadowMapAddressInfo>,
    filter_size: Node<f32>,
    num_disc_samples: Node<u32>,
    light_depth_bias: Node<f32>,
    receiver_plane_depth_bias: Node<Vec2<f32>>,
    reversed_depth: Node<bool>,
  ) -> Node<f32> {
    match self.pcf_mode {
      ShadowPCFMode::Naive => {
        // the depth bias sign is already resolved by the caller, add it to the depth
        let shadow_position = shadow_position + (val(0.), val(0.), light_depth_bias).into();
        sample_shadow_pcf_x36_by_offset(map, shadow_position, sampler, map_info.expand())
      }
      ShadowPCFMode::FixedSizePCF => sample_shadow_pcf_fixed_size(
        map,
        sampler,
        shadow_position,
        map_info,
        light_depth_bias,
        receiver_plane_depth_bias,
        self.fixed_filter_size,
        reversed_depth,
      ),
      ShadowPCFMode::OptimizedPCF => sample_shadow_pcf_optimized_fn(
        map,
        sampler,
        shadow_position,
        map_info,
        light_depth_bias,
        receiver_plane_depth_bias,
        reversed_depth,
      ),
      ShadowPCFMode::GridPCF => sample_shadow_pcf_grid_fn(
        map,
        sampler,
        shadow_position,
        map_info,
        filter_size.splat(),
        light_depth_bias,
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
        light_depth_bias,
        receiver_plane_depth_bias,
        reversed_depth,
      ),
    }
  }
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

/// the maximum kernel size of the dynamic PCF filters, in texels
pub const MAX_PCF_FILTER_SIZE: f32 = 9.0;

/// the filter kernel size of the optimized PCF
pub const OPTIMIZED_PCF_FILTER_SIZE: u32 = 3;

/// a naive PCF implementation, 3x3 grid sample with linear compare sampler
pub fn sample_shadow_pcf_x36_by_offset(
  map: BindingNode<ShaderDepthTexture2DArray>,
  shadow_position: Node<Vec3<f32>>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  info: ENode<ShadowMapAddressInfo>,
) -> Node<f32> {
  let uv = shadow_position.xy();
  let depth = shadow_position.z();
  let layer = info.layer_index;
  let mut ratio = val(0.0);

  let map_size = map.texture_dimension_2d(None).into_f32();
  let extra_scale = info.size / map_size;

  let uv = uv * extra_scale + info.offset / map_size;

  let s = 2_i32;

  for i in -1..=1 {
    for j in -1..=1 {
      let result = map
        .build_compare_sample_call(d_sampler, uv, depth)
        .with_offset((s * i, s * j).into())
        .with_array_index(layer)
        .sample();
      ratio += result;
    }
  }

  ratio / val(9.)
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

/// the world space size of one shadow map texel at the shading point, derived
/// from the projection row and the light space depth, so that the normal bias
/// scales with the shadow map resolution and the cascade coverage
///
/// the light space depth is the w component of the projected position, for
/// ortho projections it is 1 and the formula reduces to the constant texel
/// size, for perspective projections the texel size grows linearly with the
/// depth, note the projection is assumed to be symmetric
#[shader_fn]
pub fn shadow_texel_world_size(
  to_shadowmap_ndc: Node<Mat4<f32>>,
  position_in_shadow_center_without_translation_space: Node<Vec3<f32>>,
  info: Node<ShadowMapAddressInfo>,
) -> Node<f32> {
  let row0 = to_shadowmap_ndc.transpose().nth_colum(0).xyz();
  let light_space_depth = (to_shadowmap_ndc
    * (position_in_shadow_center_without_translation_space, val(1.)).into())
  .w()
  .abs();
  val(2.) * light_space_depth / (row0.length() * info.expand().size.x())
}

/// compute the world space normal offset, the normal_bias is in texel units
/// and the texel_world_size converts it to world space units
#[shader_fn]
pub fn compute_normal_offset(
  position_in_shadow_center_without_translation_space: Node<Vec3<f32>>,
  normal: Node<Vec3<f32>>,
  texel_world_size: Node<f32>,
  normal_bias: Node<f32>,
  use_n_dot_l: Node<bool>,
) -> Node<Vec3<f32>> {
  use_n_dot_l.select_branched(
    || {
      // todo， this is not correct for directional light if the light position is near the surface
      let light_dir = (-position_in_shadow_center_without_translation_space).normalize();
      let n_dot_l = normal.dot(light_dir);
      get_shadow_pos_offset_fn(n_dot_l, normal, texel_world_size, normal_bias)
    },
    || texel_world_size * normal_bias * normal,
  )
}

/// Calculates the offset to use for sampling the shadow map, based on the surface normal
/// adapted from GetShadowPosOffset in the MJP Shadows sample.
/// the texel_size parameter is the world space size of one shadow map texel, it
/// converts the offset from texel units to world units, so that the offset
/// scales with the shadow map resolution, the offset_scale is in texel units
#[shader_fn]
pub fn get_shadow_pos_offset(
  n_dot_l: Node<f32>,
  normal: Node<Vec3<f32>>,
  texel_size: Node<f32>,
  offset_scale: Node<f32>,
) -> Node<Vec3<f32>> {
  let nml_offset_scale = (val(1.) - n_dot_l).saturate();
  texel_size * offset_scale * nml_offset_scale * normal
}
