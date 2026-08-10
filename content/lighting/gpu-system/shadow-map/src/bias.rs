use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ShadowBiasBehaviorConfig {
  /// scale the normal offset by (1 - nDotL), so that the offset is smaller on the lit side
  pub use_n_dot_l_normal_offset: bool,
}

impl std::hash::Hash for ShadowBiasBehaviorConfig {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.use_n_dot_l_normal_offset.hash(state);
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

impl ShadowBias {
  pub fn new(bias: f32, normal_bias: f32) -> Self {
    Self {
      bias,
      normal_bias,
      ..Zeroable::zeroed()
    }
  }
}

pub(crate) fn apply_direct_depth_bias(
  reversed_depth: bool,
  bias: Node<f32>,
  shadow_position: Node<Vec3<f32>>,
) -> Node<Vec3<f32>> {
  let bias = if reversed_depth { bias } else { -bias };
  shadow_position + (val(0.), val(0.), bias).into()
}

/// compute the world space normal offset, the normal_bias is in texel units
/// and the texel_world_size converts it to world space units
pub fn compute_normal_offset(
  position_in_shadow_center_without_translation_space: Node<Vec3<f32>>,
  normal: Node<Vec3<f32>>,
  texel_world_size: Node<f32>,
  normal_bias: Node<f32>,
  use_n_dot_l: bool,
) -> Node<Vec3<f32>> {
  if use_n_dot_l {
    // todo, this wrong for direction light
    let light_dir = (-position_in_shadow_center_without_translation_space).normalize();
    let n_dot_l = normal.dot(light_dir);
    get_shadow_pos_offset_fn(n_dot_l, normal, texel_world_size, normal_bias)
  } else {
    texel_world_size * normal_bias * normal
  }
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
