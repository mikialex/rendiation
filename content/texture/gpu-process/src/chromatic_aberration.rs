use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, PartialEq)]
pub struct ChromaticAberration {
  pub normalized_screen_focus_point: Vec2<f32>,
  pub color_offset: Vec3<f32>,
}

impl Default for ChromaticAberration {
  fn default() -> Self {
    Self {
      normalized_screen_focus_point: Vec2::splat(0.5),
      color_offset: Vec3::new(0.0, 0.005, -0.005),
      ..Zeroable::zeroed()
    }
  }
}

/// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/demonstration/shaders/fragment/chromatic-aberration.frag
#[shader_fn]
pub fn chromatic_aberration(
  uv: Node<Vec2<f32>>,
  config: Node<ChromaticAberration>,
  color_tex: HandleNode<ShaderTexture2D>,
  sampler: HandleNode<ShaderSampler>,
) -> Node<Vec3<f32>> {
  let config = config.expand();
  let direction = uv - config.normalized_screen_focus_point;

  let r_uv = uv + direction * config.color_offset.x().splat::<Vec2<_>>();
  let g_uv = uv + direction * config.color_offset.y().splat::<Vec2<_>>();
  let b_uv = uv + direction * config.color_offset.z().splat::<Vec2<_>>();

  let r = color_tex.sample_zero_level(sampler, r_uv).x();
  let g = color_tex.sample_zero_level(sampler, g_uv).y();
  let b = color_tex.sample_zero_level(sampler, b_uv).z();

  (r, g, b).into()
}
