use crate::*;

#[shader_fn]
pub fn shader_srgb_to_linear_convert(srgb: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  (
    shader_srgb_to_linear_convert_per_channel_fn(srgb.x()),
    shader_srgb_to_linear_convert_per_channel_fn(srgb.y()),
    shader_srgb_to_linear_convert_per_channel_fn(srgb.z()),
  )
    .into()
}

#[shader_fn]
pub fn shader_srgb_to_linear_convert_per_channel(c: Node<f32>) -> Node<f32> {
  c.less_than(0.04045).select_branched(
    || c * val(0.0773993808),
    || (c * val(0.9478672986) + val(0.0521327014)).pow(2.4),
  )
}

#[shader_fn]
pub fn shader_linear_to_srgb_convert(srgb: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  (
    shader_linear_to_srgb_convert_per_channel(srgb.x()),
    shader_linear_to_srgb_convert_per_channel(srgb.y()),
    shader_linear_to_srgb_convert_per_channel(srgb.z()),
  )
    .into()
}

#[shader_fn]
pub fn shader_linear_to_srgb_convert_per_channel(c: Node<f32>) -> Node<f32> {
  c.less_than(0.0031308).select_branched(
    || c * val(12.92),
    || c.pow(1. / 2.4) * val(1.055) - val(0.055),
  )
}
