use crate::*;

/// Reconstructs a 2D boundary polyline as a minimal sequence of quadratic
/// Bézier curves.
///
/// Expects the input polyline to be tangent-continuous — sharp corners should
/// be handled by splitting the polyline at corner vertices before calling this
/// function.
///
/// Uses a greedy bottom-up strategy: starting from the first point, each
/// curve absorbs as many subsequent points as possible while keeping the
/// maximum distance from every point to the curve within
/// `error_distance_tolerance`.
pub fn reconstruct_boundary<T: Scalar>(
  points: &[Vec2<T>],
  error_distance_tolerance: T,
) -> Vec<QuadraticBezierCurve2d<T>> {
  if points.len() < 2 {
    return Vec::new();
  }

  let tolerance_sq = error_distance_tolerance * error_distance_tolerance;
  let mut curves = Vec::new();
  let mut start = 0;

  while start < points.len() - 1 {
    let mut end = start + 1;
    let mut best_p1 = Vec2::new(T::zero(), T::zero());

    while end < points.len() {
      let (p1, max_err_sq) = fit_quadratic_bezier_to_points(&points[start..=end]);
      if max_err_sq > tolerance_sq {
        break;
      }
      best_p1 = p1;
      end += 1;
    }

    curves.push(QuadraticBezierCurve2d {
      start: points[start],
      ctrl: best_p1,
      end: points[end - 1],
    });

    start = end - 1;
  }

  curves
}

/// Fit a single quadratic Bézier curve to a window of polyline points using
/// least squares.
///
/// The endpoints are pinned to the first and last point of the window.
/// Intermediate points are assigned chord-length parameters, and the
/// optimal control point is solved by minimizing Σ|Qᵢ - B(tᵢ)|².
///
/// Returns the optimal control point and the maximum squared error across
/// all points in the window.
fn fit_quadratic_bezier_to_points<T: Scalar>(points: &[Vec2<T>]) -> (Vec2<T>, T) {
  let n = points.len();
  debug_assert!(n >= 2);

  let p0 = points[0];
  let p2 = points[n - 1];

  if n == 2 {
    // Two points: the curve degenerates to a straight line when P₁ is the
    // midpoint of the chord. B(t) = (1-t)P₀ + tP₂.
    return ((p0 + p2) * T::half(), T::zero());
  }

  // Chord-length parameterization for points in this window
  let mut params = vec![T::zero(); n];
  let mut total_len = T::zero();
  for i in 1..n {
    total_len += (points[i] - points[i - 1]).length();
  }
  let degenerate_threshold: T = T::eval::<{ scalar_transmute(1e-12) }>();
  if total_len < degenerate_threshold {
    // Degenerate: all points coincident
    return (p0, T::zero());
  }
  let mut accum = T::zero();
  for i in 1..n {
    accum += (points[i] - points[i - 1]).length();
    params[i] = accum / total_len;
  }

  // Least squares: B(t) = (1-t)²P₀ + 2(1-t)t·P₁ + t²P₂
  // Define A(t) = 2(1-t)t, C(t) = (1-t)²P₀ + t²P₂
  // Then B(t) = A(t)·P₁ + C(t)
  // Minimize Σ|Qᵢ - A(tᵢ)P₁ - C(tᵢ)|² →
  //   P₁ = Σ A(tᵢ)(Qᵢ - C(tᵢ)) / Σ A(tᵢ)²
  let mut numerator = Vec2::new(T::zero(), T::zero());
  let mut denominator = T::zero();

  for i in 0..n {
    let t = params[i];
    let one_minus_t = T::one() - t;
    let a = T::two() * one_minus_t * t;
    let c = p0 * (one_minus_t * one_minus_t) + p2 * (t * t);
    numerator = numerator + (points[i] - c) * a;
    denominator += a * a;
  }

  let denom_threshold: T = T::eval::<{ scalar_transmute(1e-12) }>();
  let p1 = if denominator > denom_threshold {
    numerator / denominator
  } else {
    // Near-degenerate A(t) values (e.g. all points near endpoints):
    // fall back to a straight line
    (p0 + p2) * T::half()
  };

  // Compute maximum error across all points
  let mut max_err_sq = T::zero();
  for i in 0..n {
    let t = params[i];
    let one_minus_t = T::one() - t;
    let b = p0 * (one_minus_t * one_minus_t) + p1 * (T::two() * one_minus_t * t) + p2 * (t * t);
    let err_sq = (points[i] - b).length2();
    if err_sq > max_err_sq {
      max_err_sq = err_sq;
    }
  }

  (p1, max_err_sq)
}
