use crate::*;

/// Adaptively tessellates a rational Bézier curve into a polyline.
///
/// The curve is recursively subdivided at its parametric midpoint until the
/// control polygon of each segment deviates from its chord by at most
/// `error_distance_tolerance`. The result is a list of 3D points that, when
/// connected sequentially, approximate the curve within the given tolerance.
///
/// # Termination guarantee
///
/// The tolerance is clamped to `MIN_TOLERANCE` and the recursion depth is
/// capped at `MAX_DEPTH`, so the function always returns in bounded time
/// even for pathological input.
pub fn adaptive_tessellate_bezier_curve<T: Scalar>(
  curve: &RationalBezierCurve3d<T>,
  error_distance_tolerance: T,
) -> Vec<Vec3<T>> {
  let tolerance = if error_distance_tolerance < min_tolerance::<T>() {
    min_tolerance::<T>()
  } else {
    error_distance_tolerance
  };
  let tolerance_sq = tolerance * tolerance;
  let mut result = Vec::new();
  result.push(curve.evaluate(T::zero()));
  tessellate_recursive(&curve, tolerance_sq, &mut result, 0);
  result
}

/// Recursively tessellates a curve segment.
///
/// **Invariant**: `out` already contains the start point of `curve` when
/// this function is called. Each call either emits the end point (if the
/// curve is flat enough) or subdivides and recurses into both halves.
///
/// The left half is processed first to maintain spatial ordering. Because
/// the left's end equals the right's start, the shared midpoint appears
/// only once in the output.
fn tessellate_recursive<T: Scalar>(
  curve: &RationalBezierCurve3d<T>,
  tolerance_sq: T,
  out: &mut Vec<Vec3<T>>,
  depth: u32,
) {
  if depth >= MAX_DEPTH || is_flat_enough(curve, tolerance_sq) {
    out.push(curve.evaluate(T::one()));
    return;
  }
  let (left_cp, right_cp) = subdivide_bezier(curve.control_points());
  let degree = curve.degree();
  let left = RationalBezierCurve3d::new(left_cp, degree);
  let right = RationalBezierCurve3d::new(right_cp, degree);
  tessellate_recursive(&left, tolerance_sq, out, depth + 1);
  tessellate_recursive(&right, tolerance_sq, out, depth + 1);
}

/// Splits a Bézier curve at `t = 0.5` using the de Casteljau algorithm.
///
/// The de Casteljau triangular scheme produces both sub-curve control
/// polygons as a by-product:
/// - **Left sub-curve**: the first point from each row of the triangle.
/// - **Right sub-curve**: the last point from each row (reversed).
///
/// Both sub-curves share the midpoint `C(0.5)` — the left curve's end is
/// the right curve's start. This shared point is naturally deduplicated by
/// the recursive tessellation invariant.
///
/// Returns `(left_control_points, right_control_points)`.
fn subdivide_bezier<T: Scalar>(cp: &[Vec4<T>]) -> (Vec<Vec4<T>>, Vec<Vec4<T>>) {
  let n = cp.len();
  let t = T::half();
  let one_minus_t = T::one() - t;
  let mut left = vec![Vec4::new(T::zero(), T::zero(), T::zero(), T::zero()); n];
  let mut right = vec![Vec4::new(T::zero(), T::zero(), T::zero(), T::zero()); n];
  let mut q = cp.to_vec();
  left[0] = cp[0];
  for r in 1..n {
    right[r - 1] = q[n - r];
    for i in 0..(n - r) {
      q[i] = q[i] * one_minus_t + q[i + 1] * t;
    }
    left[r] = q[0];
  }
  right[n - 1] = q[0];
  right.reverse();
  (left, right)
}

/// Decides whether a rational Bézier curve is "flat enough" to be replaced
/// by its chord segment.
///
/// The test:
/// 1. Rejects the curve (returns `false`) if any control-point weight is
///    near zero — the perspective divide would be unstable, so we subdivide
///    and hope the sub-curves have better-behaved weights.
/// 2. Projects every control point to Euclidean 3D by dividing (x,y,z) by w.
/// 3. Checks the squared distance from each *interior* projected control
///    point to the chord segment between the first and last projected points.
///
/// Because Bézier curves lie within the convex hull of their control points,
/// bounding the control polygon also bounds the curve. For rational curves
/// this property holds in homogeneous space; projecting to 3D gives a
/// slightly conservative but safe estimate.
fn is_flat_enough<T: Scalar>(curve: &RationalBezierCurve3d<T>, tolerance_sq: T) -> bool {
  let cp = curve.control_points();
  let min_w = min_weight::<T>();
  for p in cp {
    if p.w.abs() < min_w {
      return false;
    }
  }
  let projected: Vec<Vec3<T>> = cp
    .iter()
    .map(|p| Vec3::new(p.x / p.w, p.y / p.w, p.z / p.w))
    .collect();
  let p0 = projected[0];
  let pn = projected[projected.len() - 1];
  for pi in &projected[1..projected.len() - 1] {
    if point_to_segment_distance_sq(*pi, p0, pn) > tolerance_sq {
      return false;
    }
  }
  true
}

/// Squared perpendicular distance from a point `p` to the line segment `ab`.
///
/// The projection parameter is clamped to [0, 1] so that the distance is
/// measured to the finite segment rather than to the infinite chord line.
/// This is important for the flatness test: control points that extend
/// beyond the segment endpoints would pass a line-distance check but still
/// cause the curve to bow outside the segment region.
pub(crate) fn point_to_segment_distance_sq<T: Scalar>(p: Vec3<T>, a: Vec3<T>, b: Vec3<T>) -> T {
  let ab = b - a;
  let ap = p - a;
  let ab_len_sq = ab.length2();
  let near_zero: T = T::eval::<{ scalar_transmute(1e-12) }>();
  if ab_len_sq < near_zero {
    return ap.length2();
  }
  let t_raw = ap.dot(ab) / ab_len_sq;
  let t = if t_raw < T::zero() {
    T::zero()
  } else if t_raw > T::one() {
    T::one()
  } else {
    t_raw
  };
  let closest = a + ab * t;
  (p - closest).length2()
}

/// Floor for the error tolerance to prevent infinite recursion when the
/// caller passes zero or a very small tolerance.
fn min_tolerance<T: Scalar>() -> T {
  T::eval::<{ scalar_transmute(1e-7) }>()
}

/// Hard limit on recursion depth — guarantees termination regardless of
/// tolerance value or pathological curve shapes (e.g. cusps).
const MAX_DEPTH: u32 = 20;

/// Weight threshold for the perspective divide. If any control-point weight
/// is below this value the flatness test forces a subdivision so that the
/// projection to 3D remains numerically stable.
fn min_weight<T: Scalar>() -> T {
  T::eval::<{ scalar_transmute(1e-10) }>()
}
