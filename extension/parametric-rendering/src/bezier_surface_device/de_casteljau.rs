use rendiation_algebra::*;
use rendiation_shader_api::*;

/// de Casteljau evaluation of a rational Bézier curve of degree 1 (2 control points).
///
/// Returns `p0 * (1-t) + p1 * t` in homogeneous coordinates.
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
///
/// Two-step reduction: 3 → 2 → 1.
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
///
/// Three-step reduction: 4 → 3 → 2 → 1. Fully unrolled — 6 FMAs.
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
