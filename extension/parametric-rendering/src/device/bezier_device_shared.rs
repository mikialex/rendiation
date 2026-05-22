use crate::*;

/// Maximum supported degree for Bernstein evaluation on GPU.
pub const MAX_GPU_DEGREE: usize = 14;

/// Binomial coefficients C(n,k) for n = 0..=14, k = 0..=n.
///
/// Layout: 15 rows × 16 columns, row-major.
/// Index: `BINOMIAL_COEFFICIENTS[(degree - 1) * 16 + k]` for degree ∈ [1,14], k ∈ [0, degree].
/// Row 0 stores C(1,*); row 13 stores C(14,*). Unused entries are 0.
#[rustfmt::skip]
pub const BINOMIAL_COEFFICIENTS: [f32; 224] = [
  // degree 1
  1., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 2
  1., 2., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 3
  1., 3., 3., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 4
  1., 4., 6., 4., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 5
  1., 5., 10., 10., 5., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 6
  1., 6., 15., 20., 15., 6., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 7
  1., 7., 21., 35., 35., 21., 7., 1., 0., 0., 0., 0., 0., 0., 0., 0.,
  // degree 8
  1., 8., 28., 56., 70., 56., 28., 8., 1., 0., 0., 0., 0., 0., 0., 0.,
  // degree 9
  1., 9., 36., 84., 126., 126., 84., 36., 9., 1., 0., 0., 0., 0., 0., 0.,
  // degree 10
  1., 10., 45., 120., 210., 252., 210., 120., 45., 10., 1., 0., 0., 0., 0., 0.,
  // degree 11
  1., 11., 55., 165., 330., 462., 462., 330., 165., 55., 11., 1., 0., 0., 0., 0.,
  // degree 12
  1., 12., 66., 220., 495., 792., 924., 792., 495., 220., 66., 12., 1., 0., 0., 0.,
  // degree 13
  1., 13., 78., 286., 715., 1287., 1716., 1716., 1287., 715., 286., 78., 13., 1., 0., 0.,
  // degree 14
  1., 14., 91., 364., 1001., 2002., 3003., 3432., 3003., 2002., 1001., 364., 91., 14., 1., 0.,
];

/// de Casteljau evaluation of a rational Bézier curve of degree 1 (2 control points).
#[shader_fn]
pub fn de_casteljau_curve_deg1(
  p0: Node<Vec4<f32>>,
  p1: Node<Vec4<f32>>,
  t: Node<f32>,
) -> Node<Vec4<f32>> {
  let one_minus_t = val(1.0) - t;
  p0 * one_minus_t + p1 * t
}

/// de Casteljau evaluation of a rational Bézier curve of degree 2 (3 control points).
#[shader_fn]
pub fn de_casteljau_curve_deg2(
  p0: Node<Vec4<f32>>,
  p1: Node<Vec4<f32>>,
  p2: Node<Vec4<f32>>,
  t: Node<f32>,
) -> Node<Vec4<f32>> {
  let one_minus_t = val(1.0) - t;
  let q0 = p0 * one_minus_t + p1 * t;
  let q1 = p1 * one_minus_t + p2 * t;
  q0 * one_minus_t + q1 * t
}

/// de Casteljau evaluation of a rational Bézier curve of degree 3 (4 control points).
#[shader_fn]
pub fn de_casteljau_curve_deg3(
  p0: Node<Vec4<f32>>,
  p1: Node<Vec4<f32>>,
  p2: Node<Vec4<f32>>,
  p3: Node<Vec4<f32>>,
  t: Node<f32>,
) -> Node<Vec4<f32>> {
  let one_minus_t = val(1.0) - t;
  let q0 = p0 * one_minus_t + p1 * t;
  let q1 = p1 * one_minus_t + p2 * t;
  let q2 = p2 * one_minus_t + p3 * t;
  let r0 = q0 * one_minus_t + q1 * t;
  let r1 = q1 * one_minus_t + q2 * t;
  r0 * one_minus_t + r1 * t
}
