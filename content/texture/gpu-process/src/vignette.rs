use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, PartialEq)]
pub struct VignetteEffect {
  pub mid_point: f32,
  pub radius: f32,
  pub aspect: f32,
  pub feather: f32,
  pub color: Vec3<f32>,
}

impl Default for VignetteEffect {
  fn default() -> Self {
    Self {
      mid_point: 1.0,
      radius: 1.0,
      aspect: 1.0,
      feather: 0.5,
      color: Vec3::zero(),
      ..Zeroable::zeroed()
    }
  }
}

/// from filament
#[shader_fn]
pub fn compute_vignette(
  uv: Node<Vec2<f32>>,
  config: Node<VignetteEffect>,
  color: Node<Vec3<f32>>,
) -> Node<Vec3<f32>> {
  let config = config.expand();
  let distance = (uv - val(0.5).splat()).abs() * config.mid_point.splat::<Vec2<f32>>();
  let distance = vec2_node((distance.x() * config.aspect, distance.y()));
  let distance = distance.saturate().pow(config.radius.splat());

  let amount = (val(1.) - distance.dot(distance))
    .saturate()
    .pow(config.feather * val(5.0))
    .splat::<Vec3<f32>>();
  color * amount.mix(config.color, val(Vec3::one()))
}
