use rendiation_shader_api::*;

pub mod cube;
pub mod normal_mapping;
pub mod plane;
pub mod sampling;
pub mod sky;
pub mod z_order;

pub fn shader_uv_space_to_render_space(
  ndc_space_to_render_space: Node<Mat4<f32>>,
  uv: Node<Vec2<f32>>,
  ndc_depth: Node<f32>,
) -> Node<Vec3<f32>> {
  let xy = uv * val(2.) - val(Vec2::one());
  let xy = xy * val(Vec2::new(1., -1.));
  let ndc = (xy, ndc_depth, val(1.)).into();
  let render = ndc_space_to_render_space * ndc;
  render.xyz() / render.w().splat()
}

pub fn shader_render_space_to_uv_space(
  render_space_to_ndc_space: Node<Mat4<f32>>,
  render: Node<Vec3<f32>>,
) -> (Node<Vec2<f32>>, Node<f32>) {
  let clip = render_space_to_ndc_space * (render, val(1.)).into();
  let ndc = clip.xyz() / clip.w().splat();
  let uv = ndc.xy() * val(Vec2::new(0.5, -0.5)) + val(Vec2::splat(0.5));
  (uv, ndc.z())
}

// todo, fix upstream
pub fn shader_identity_mat4() -> Node<Mat4<f32>> {
  let a = val(Vec4::new(1., 0., 0., 0.));
  let b = val(Vec4::new(0., 1., 0., 0.));
  let c = val(Vec4::new(0., 0., 1., 0.));
  let d = val(Vec4::new(0., 0., 0., 1.));
  (a, b, c, d).into()
}

// todo, fix upstream
pub fn shader_identity_mat3() -> Node<Mat3<f32>> {
  let a = val(Vec3::new(1., 0., 0.));
  let b = val(Vec3::new(0., 1., 0.));
  let c = val(Vec3::new(0., 0., 1.));
  (a, b, c).into()
}

pub fn shader_identity_mat2() -> Node<Mat2<f32>> {
  let a = val(Vec2::new(1., 0.));
  let b = val(Vec2::new(0., 1.));
  (a, b).into()
}

#[shader_fn]
pub fn mat4_equal(mat: Node<Mat4<f32>>, ref_mat: Node<Mat4<f32>>) -> Node<bool> {
  let x = mat.x().equals(ref_mat.x()).all();
  let y = mat.y().equals(ref_mat.y()).all();
  let z = mat.z().equals(ref_mat.z()).all();
  let w = mat.w().equals(ref_mat.w()).all();
  x.and(y).and(z).and(w)
}

#[shader_fn]
pub fn mat3_equal(mat: Node<Mat3<f32>>, ref_mat: Node<Mat3<f32>>) -> Node<bool> {
  let x = mat.x().equals(ref_mat.x()).all();
  let y = mat.y().equals(ref_mat.y()).all();
  let z = mat.z().equals(ref_mat.z()).all();
  x.and(y).and(z)
}

#[shader_fn]
pub fn mat2_equal(mat: Node<Mat2<f32>>, ref_mat: Node<Mat2<f32>>) -> Node<bool> {
  let x = mat.x().equals(ref_mat.x()).all();
  let y = mat.y().equals(ref_mat.y()).all();
  x.and(y)
}

/// impl polygon offset in fragment shader
#[shader_fn]
pub fn shader_depth_bias(
  fragment_depth: Node<f32>,
  constant: Node<i32>,
  slope_scale: Node<f32>,
  clamp: Node<f32>,
) -> Node<f32> {
  let (_, exp) = fragment_depth.frexp();
  let dx = fragment_depth.dpdx().abs();
  let dy = fragment_depth.dpdy().abs();
  let max_slope = dx.max(dy);
  let r = (exp - val(23i32)).into_f32().exp2(); // 23 is f32 mantissa bit count.
  let bias = constant.into_f32() * r + slope_scale * max_slope;

  let bias = bias.make_local_var();
  if_by(clamp.greater_than(0.), || {
    bias.store(bias.load().min(clamp));
  })
  .else_if(clamp.less_than(0.), || {
    bias.store(bias.load().max(clamp));
  })
  .else_over();

  bias.load()
}
