use crate::*;

/// A rational Bézier curve in 3D of arbitrary degree.
///
/// Defined by a sequence of weighted control points in homogeneous coordinates,
/// evaluated via the de Casteljau algorithm.
///
/// Control points are stored as `Vec4<T>` where `(x, y, z)` are the geometric
/// coordinates pre-multiplied by the weight `w`.
#[derive(Debug, Clone)]
pub struct RationalBezierCurve3d<T> {
  control_points: Vec<Vec4<T>>,
  degree: usize,
}

impl<T: Scalar> RationalBezierCurve3d<T> {
  /// Create a new rational Bézier curve.
  ///
  /// * `control_points` - Weighted control points (length = `degree + 1`).
  /// * `degree` - Degree (≥ 1).
  pub fn new(control_points: Vec<Vec4<T>>, degree: usize) -> Self {
    debug_assert_eq!(control_points.len(), degree + 1);
    debug_assert!(degree >= 1);
    Self {
      control_points,
      degree,
    }
  }

  /// Create a rational Bézier curve from unweighted 3D control points
  /// (all weights = 1). The result is a non-rational Bézier curve in
  /// homogeneous form.
  pub fn from_unweighted(points: Vec<Vec3<T>>, degree: usize) -> Self {
    let control_points = points
      .into_iter()
      .map(|p| Vec4::new(p.x, p.y, p.z, T::one()))
      .collect();
    Self::new(control_points, degree)
  }

  // --- Accessors ---

  pub fn degree(&self) -> usize {
    self.degree
  }
  pub fn control_points(&self) -> &[Vec4<T>] {
    &self.control_points
  }
  pub fn control_points_mut(&mut self) -> &mut [Vec4<T>] {
    &mut self.control_points
  }
  pub fn control_point(&self, idx: usize) -> Vec4<T> {
    self.control_points[idx]
  }

  // --- Evaluation ---

  /// Evaluate the curve point at parameter `t`, in `[0, 1]`.
  pub fn evaluate(&self, t: T) -> Vec3<T> {
    let result = Self::de_casteljau(&self.control_points, t);
    Vec3::new(
      result.x / result.w,
      result.y / result.w,
      result.z / result.w,
    )
  }

  /// Evaluate the curve point and first-order derivative.
  ///
  /// Returns `(point, derivative)` where `derivative = dS/dt`.
  /// Uses the hodograph (forward difference) of control points.
  pub fn evaluate_partial(&self, t: T) -> (Vec3<T>, Vec3<T>) {
    let a = Self::de_casteljau(&self.control_points, t);

    // Hodograph: forward differences, degree (n-1)
    let du_cp: Vec<Vec4<T>> = (0..self.degree)
      .map(|i| self.control_points[i + 1] - self.control_points[i])
      .collect();
    let au_raw = Self::de_casteljau(&du_cp, t);
    let deg_t = T::from(self.degree).expect("degree should be representable");
    let au = Vec4::new(
      au_raw.x * deg_t,
      au_raw.y * deg_t,
      au_raw.z * deg_t,
      au_raw.w * deg_t,
    );

    let w = a.w;
    let w2 = w * w;

    let point = Vec3::new(a.x / w, a.y / w, a.z / w);
    let deriv = Vec3::new(
      (au.x * w - a.x * au.w) / w2,
      (au.y * w - a.y * au.w) / w2,
      (au.z * w - a.z * au.w) / w2,
    );

    (point, deriv)
  }
}

// --- Private helpers ---

impl<T: Scalar> RationalBezierCurve3d<T> {
  /// Evaluate a rational Bézier curve in homogeneous coordinates
  /// using the de Casteljau algorithm. Returns `Vec4<T>` on the curve.
  fn de_casteljau(points: &[Vec4<T>], t: T) -> Vec4<T> {
    let n = points.len();
    debug_assert!(n > 0, "de Casteljau requires at least one control point");
    let mut q = points.to_vec();
    let one_minus_t = T::one() - t;
    for r in 1..n {
      for i in 0..n - r {
        q[i] = q[i] * one_minus_t + q[i + 1] * t;
      }
    }
    q[0]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_curve_deg3() -> RationalBezierCurve3d<f32> {
    RationalBezierCurve3d::from_unweighted(
      vec![
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(-0.5, 1.0, 0.3),
        Vec3::new(0.5, 1.0, -0.3),
        Vec3::new(1.0, 0.0, 0.0),
      ],
      3,
    )
  }

  #[test]
  fn evaluate_endpoints() {
    let curve = test_curve_deg3();
    let p0 = curve.evaluate(0.0);
    let p1 = curve.evaluate(1.0);
    assert!((p0.x + 1.0).abs() < 1e-6);
    assert!((p1.x - 1.0).abs() < 1e-6);
  }

  #[test]
  fn evaluate_partial_consistency() {
    let curve = test_curve_deg3();
    let (pt, dt) = curve.evaluate_partial(0.5);
    // Derivative should be non-zero for this curve
    assert!(dt.length() > 0.1);
    // Point should be finite
    assert!(pt.length() < 10.0);
  }
}
