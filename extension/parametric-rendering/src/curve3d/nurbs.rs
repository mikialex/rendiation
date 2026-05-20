use crate::*;

/// A NURBS (Non-Uniform Rational B-Spline) curve in 3D of arbitrary degree.
///
/// The curve is defined by a sequence of control points with weights (in homogeneous
/// coordinates), a knot vector, and a degree.
///
/// Control points are stored as `Vec4<T>` where `(x, y, z)` are the geometric
/// coordinates and `w` is the weight.
#[derive(Debug, Clone)]
pub struct NurbsCurve3d<T> {
  control_points: Vec<Vec4<T>>,
  count: usize,
  degree: usize,
  knots: Vec<T>,
}

impl<T: Scalar> NurbsCurve3d<T> {
  /// Create a new NURBS curve.
  ///
  /// * `control_points` - Weighted control points (length = `count`).
  /// * `count` - Number of control points.
  /// * `degree` - Degree (≥ 1).
  /// * `knots` - Knot vector (length = `count + degree + 1`).
  pub fn new(control_points: Vec<Vec4<T>>, count: usize, degree: usize, knots: Vec<T>) -> Self {
    debug_assert_eq!(control_points.len(), count);
    debug_assert!(
      knots.len() >= count + degree + 1,
      "knots too short: {} < {} (count={} + degree={} + 1)",
      knots.len(),
      count + degree + 1,
      count,
      degree
    );
    debug_assert!(degree >= 1);
    Self {
      control_points,
      count,
      degree,
      knots,
    }
  }

  /// Create a NURBS curve from unweighted 3D control points (all weights = 1),
  /// making it a B-spline curve in homogeneous form.
  pub fn from_unweighted(points: Vec<Vec3<T>>, degree: usize, knots: Vec<T>) -> Self {
    let count = points.len();
    let control_points: Vec<Vec4<T>> = points
      .into_iter()
      .map(|p| Vec4::new(p.x, p.y, p.z, T::one()))
      .collect();
    Self::new(control_points, count, degree, knots)
  }

  // --- Accessors ---

  pub fn count(&self) -> usize {
    self.count
  }
  pub fn degree(&self) -> usize {
    self.degree
  }
  pub fn knots(&self) -> &[T] {
    &self.knots
  }
  pub fn control_points(&self) -> &[Vec4<T>] {
    &self.control_points
  }

  /// Valid parameter range: `[knots[degree], knots[count]]`.
  pub fn domain(&self) -> (T, T) {
    (self.knots[self.degree], self.knots[self.count])
  }

  // --- Evaluation ---

  /// Evaluate the curve point at parameter `t`.
  pub fn evaluate(&self, t: T) -> Vec3<T> {
    let span = Self::find_knot_span(self.count - 1, self.degree, t, &self.knots);
    let n = Self::basis_functions(span, t, self.degree, &self.knots);

    let mut sum = Vec4::splat(T::zero());
    for i in 0..=self.degree {
      let idx = span - self.degree + i;
      let basis = n[i];
      let cp = self.control_points[idx];
      sum.x += basis * cp.x;
      sum.y += basis * cp.y;
      sum.z += basis * cp.z;
      sum.w += basis * cp.w;
    }
    Vec3::new(sum.x / sum.w, sum.y / sum.w, sum.z / sum.w)
  }

  /// Evaluate the curve point and first-order derivative.
  ///
  /// Returns `(point, derivative)` where `derivative = dS/dt`.
  pub fn evaluate_partial(&self, t: T) -> (Vec3<T>, Vec3<T>) {
    let span = Self::find_knot_span(self.count - 1, self.degree, t, &self.knots);
    let (n, nd) = Self::basis_functions_and_derivatives(span, t, self.degree, &self.knots);

    let mut a = Vec4::splat(T::zero());
    let mut ad = Vec4::splat(T::zero());

    for i in 0..=self.degree {
      let idx = span - self.degree + i;
      let cp = self.control_points[idx];

      let basis = n[i];
      a.x += basis * cp.x;
      a.y += basis * cp.y;
      a.z += basis * cp.z;
      a.w += basis * cp.w;

      let dbasis = nd[i];
      ad.x += dbasis * cp.x;
      ad.y += dbasis * cp.y;
      ad.z += dbasis * cp.z;
      ad.w += dbasis * cp.w;
    }

    let w = a.w;
    let w2 = w * w;

    let point = Vec3::new(a.x / w, a.y / w, a.z / w);
    let deriv = Vec3::new(
      (ad.x * w - a.x * ad.w) / w2,
      (ad.y * w - a.y * ad.w) / w2,
      (ad.z * w - a.z * ad.w) / w2,
    );

    (point, deriv)
  }

  /// Decompose the NURBS curve into mathematically equivalent rational Bézier
  /// curve segments by inserting all interior knots to full multiplicity.
  ///
  /// Returns a vector of Bézier curve segments. If the NURBS curve has no
  /// interior knots (already in Bézier form), the result is a single segment.
  pub fn to_bezier_curves(&self) -> Vec<RationalBezierCurve3d<T>> {
    let p = self.degree;
    let mut cp = self.control_points.clone();
    let mut knots = self.knots.clone();

    // Insert interior knots to full multiplicity (p)
    let knot_info = Self::interior_knot_info(&knots, p);
    for (knot, mult) in knot_info {
      for _ in mult..p {
        Self::insert_knot(&mut cp, &mut knots, p, knot);
      }
    }

    let total_cp = cp.len();
    let segments = (total_cp - 1) / p;
    let mut curves = Vec::with_capacity(segments);

    for s in 0..segments {
      let start = s * p;
      let segment_cp: Vec<Vec4<T>> = cp[start..=start + p].to_vec();
      curves.push(RationalBezierCurve3d::new(segment_cp, p));
    }

    curves
  }
}

// --- Private helpers ---

impl<T: Scalar> NurbsCurve3d<T> {
  /// Find the knot span index i such that `knots[i] <= t < knots[i+1]`.
  /// `n` is the index of the last control point (`count - 1`), `p` is the degree.
  fn find_knot_span(n: usize, p: usize, t: T, knots: &[T]) -> usize {
    if t >= knots[n + 1] {
      return n;
    }
    if t <= knots[p] {
      return p;
    }

    let mut low = p;
    let mut high = n + 1;
    let mut mid = (low + high) >> 1;

    while t < knots[mid] || t >= knots[mid + 1] {
      if t < knots[mid] {
        high = mid;
      } else {
        low = mid;
      }
      mid = (low + high) >> 1;
    }

    mid
  }

  /// Compute all non-zero basis functions `N_{span-p+i, p}(t)` for `i = 0..=p`.
  /// Algorithm A2.2 from *The NURBS Book* (Piegl & Tiller).
  fn basis_functions(span: usize, t: T, p: usize, knots: &[T]) -> Vec<T> {
    let mut n = vec![T::zero(); p + 1];
    n[0] = T::one();

    let mut left = vec![T::zero(); p + 1];
    let mut right = vec![T::zero(); p + 1];

    for j in 1..=p {
      left[j] = t - knots[span + 1 - j];
      right[j] = knots[span + j] - t;
      let mut saved = T::zero();
      for r in 0..j {
        let denom = right[r + 1] + left[j - r];
        let temp = n[r] / denom;
        n[r] = saved + right[r + 1] * temp;
        saved = left[j - r] * temp;
      }
      n[j] = saved;
    }

    n
  }

  /// Compute basis functions `N_{i,p}(t)` and their first derivatives `N'_{i,p}(t)`.
  ///
  /// Returns `(basis_values, derivative_values)`.
  fn basis_functions_and_derivatives(span: usize, t: T, p: usize, knots: &[T]) -> (Vec<T>, Vec<T>) {
    let n = Self::basis_functions(span, t, p, knots);

    if p == 0 {
      return (n, vec![T::zero(); 1]);
    }

    let n_low = Self::basis_functions(span, t, p - 1, knots);
    let mut nd = vec![T::zero(); p + 1];
    let p_t = T::from(p).expect("degree should be representable");

    for i in 0..=p {
      let k = span - p + i;

      let denom1 = knots[k + p] - knots[k];
      if i > 0 && denom1 > T::zero() {
        nd[i] += p_t * n_low[i - 1] / denom1;
      }

      let denom2 = knots[k + p + 1] - knots[k + 1];
      if i < p && denom2 > T::zero() {
        nd[i] -= p_t * n_low[i] / denom2;
      }
    }

    (n, nd)
  }

  /// Collect distinct interior knot values with their current multiplicity.
  fn interior_knot_info(knots: &[T], degree: usize) -> Vec<(T, usize)> {
    if knots.len() <= 2 * (degree + 1) {
      return vec![];
    }
    let interior = &knots[degree + 1..knots.len() - degree - 1];
    let mut result = Vec::new();
    let mut i = 0;
    while i < interior.len() {
      let cur = interior[i];
      let mut count = 1;
      while i + count < interior.len() && interior[i + count] == cur {
        count += 1;
      }
      result.push((cur, count));
      i += count;
    }
    result
  }

  /// Insert a knot `t` into the curve (Algorithm A5.1 from *The NURBS Book*).
  fn insert_knot(cp: &mut Vec<Vec4<T>>, knots: &mut Vec<T>, degree: usize, t: T) {
    let n = cp.len() - 1;
    let k = Self::find_knot_span(n, degree, t, knots);

    let start = (k + 1).saturating_sub(degree);
    let mut alphas = vec![T::zero(); k + 1];
    for i in start..=k {
      let denom = knots[i + degree] - knots[i];
      alphas[i] = if denom > T::zero() {
        (t - knots[i]) / denom
      } else {
        T::zero()
      };
    }

    knots.insert(k + 1, t);

    let old_len = n + 1;
    let mut new_cp = Vec::with_capacity(old_len + 1);

    // Unchanged: indices 0 .. start
    for i in 0..start {
      new_cp.push(cp[i]);
    }
    // Modified: indices start .. k
    for i in start..=k {
      let alpha = alphas[i];
      new_cp.push(cp[i] * alpha + cp[i - 1] * (T::one() - alpha));
    }
    // Shifted: indices k+1 .. n+1
    for i in k + 1..=old_len {
      new_cp.push(cp[i - 1]);
    }

    *cp = new_cp;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn points_close(a: Vec3<f32>, b: Vec3<f32>, eps: f32) -> bool {
    (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
  }

  fn test_curve() -> NurbsCurve3d<f32> {
    let points = vec![
      Vec3::new(-2.0, 0.0, 0.0),
      Vec3::new(-1.0, 1.5, 0.5),
      Vec3::new(1.0, 1.5, -0.5),
      Vec3::new(2.0, 0.0, 0.0),
    ];
    // Clamped uniform knot vector for degree 3, 4 CPs
    let knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    NurbsCurve3d::from_unweighted(points, 3, knots)
  }

  #[test]
  fn evaluate_endpoints() {
    let curve = test_curve();
    let p0 = curve.evaluate(0.0);
    let p1 = curve.evaluate(1.0);
    assert!(points_close(p0, Vec3::new(-2.0, 0.0, 0.0), 1e-5));
    assert!(points_close(p1, Vec3::new(2.0, 0.0, 0.0), 1e-5));
  }

  #[test]
  fn bezier_form_to_single_curve() {
    let curve = test_curve(); // clamped knots → Bézier form
    let curves = curve.to_bezier_curves();
    assert_eq!(curves.len(), 1);

    let bezier = &curves[0];
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
      let np = curve.evaluate(t);
      let bp = bezier.evaluate(t);
      assert!(
        points_close(np, bp, 1e-5),
        "mismatch at t={t}: NURBS {np:?} vs Bézier {bp:?}"
      );
    }
  }

  #[test]
  fn with_interior_knot_decomposition() {
    // 4 CPs, degree 2, with one interior knot at 0.5
    let points = vec![
      Vec3::new(-2.0, 0.0, 0.0),
      Vec3::new(-0.5, 1.0, 0.3),
      Vec3::new(0.5, 1.0, -0.3),
      Vec3::new(1.5, 0.0, 0.1),
    ];
    // count=4, degree=2 → 7 knots; one interior knot at 0.5
    let knots = vec![0., 0., 0., 0.5, 1., 1., 1.];
    let curve = NurbsCurve3d::from_unweighted(points, 2, knots);

    let curves = curve.to_bezier_curves();
    assert_eq!(curves.len(), 2, "should produce 2 Bézier segments");

    // Check evaluation matches
    for t in [0.0, 0.2, 0.45, 0.55, 0.8, 1.0] {
      let np = curve.evaluate(t);
      // Find which segment and local parameter
      let seg = if t <= 0.5 { 0 } else { 1 };
      let lt = if t <= 0.5 { t / 0.5 } else { (t - 0.5) / 0.5 };
      let bp = curves[seg].evaluate(lt);
      assert!(
        points_close(np, bp, 1e-4),
        "mismatch at t={t}: NURBS {np:?} vs Bézier[{seg}]@lt={lt:.3} {bp:?}"
      );
    }

    // Continuity at knot boundary
    let left_end = curves[0].evaluate(1.0);
    let right_start = curves[1].evaluate(0.0);
    assert!(
      points_close(left_end, right_start, 1e-5),
      "C0 discontinuity at knot"
    );
  }
}
