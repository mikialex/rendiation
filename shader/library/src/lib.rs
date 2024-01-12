use rendiation_shader_api::*;

pub mod cube;
pub mod normal_mapping;
pub mod sampling;

pub fn shader_uv_space_to_world_space(
  view_projection_inv: Node<Mat4<f32>>,
  uv: Node<Vec2<f32>>,
  ndc_depth: Node<f32>,
) -> Node<Vec3<f32>> {
  let xy = uv * val(2.) - val(Vec2::one());
  let xy = xy * val(Vec2::new(1., -1.));
  let ndc = (xy, ndc_depth, val(1.)).into();
  let world = view_projection_inv * ndc;
  world.xyz() / world.w().splat()
}

pub fn shader_world_space_to_uv_space(
  view_projection: Node<Mat4<f32>>,
  world: Node<Vec3<f32>>,
) -> (Node<Vec2<f32>>, Node<f32>) {
  let clip = view_projection * (world, val(1.)).into();
  let ndc = clip.xyz() / clip.w().splat();
  let uv = ndc.xy() * val(Vec2::new(0.5, -0.5)) + val(Vec2::splat(0.5));
  (uv, ndc.z())
}
