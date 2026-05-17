use rendiation_algebra::*;
use rendiation_mesh_generator::ParametricSurface;

/// A rational Bézier surface of arbitrary degree.
///
/// Defined by a grid of weighted control points in homogeneous coordinates,
/// evaluated via the de Casteljau algorithm.
///
/// Control points are stored as `Vec4<T>` where `(x, y, z)` are the geometric
/// coordinates pre-multiplied by the weight `w`. Grid is row-major:
/// `index = v * (u_degree + 1) + u`.
#[derive(Debug, Clone)]
pub struct RationalBezierSurface<T> {
  control_points: Vec<Vec4<T>>,
  u_degree: usize,
  v_degree: usize,
}

impl<T: Scalar> RationalBezierSurface<T> {
  /// Create a new rational Bézier surface.
  ///
  /// * `control_points` - Weighted control points in row-major order
  ///   (length = `(u_degree + 1) * (v_degree + 1)`).
  /// * `u_degree` / `v_degree` - Degree in each direction (≥ 1).
  pub fn new(control_points: Vec<Vec4<T>>, u_degree: usize, v_degree: usize) -> Self {
    let expected = (u_degree + 1) * (v_degree + 1);
    debug_assert_eq!(control_points.len(), expected);
    debug_assert!(u_degree >= 1);
    debug_assert!(v_degree >= 1);
    Self { control_points, u_degree, v_degree }
  }

  /// Create a rational Bézier surface from unweighted 3D control points
  /// (all weights = 1). The result is a non-rational Bézier surface in
  /// homogeneous form.
  pub fn from_unweighted(points: Vec<Vec3<T>>, u_degree: usize, v_degree: usize) -> Self {
    let control_points = points
      .into_iter()
      .map(|p| Vec4::new(p.x, p.y, p.z, T::one()))
      .collect();
    Self::new(control_points, u_degree, v_degree)
  }

  // --- Accessors ---

  pub fn u_degree(&self) -> usize { self.u_degree }
  pub fn v_degree(&self) -> usize { self.v_degree }
  pub fn u_count(&self) -> usize { self.u_degree + 1 }
  pub fn v_count(&self) -> usize { self.v_degree + 1 }
  pub fn control_points(&self) -> &[Vec4<T>] { &self.control_points }

  pub fn control_point(&self, u_idx: usize, v_idx: usize) -> Vec4<T> {
    self.control_points[v_idx * self.u_count() + u_idx]
  }

  // --- Evaluation ---

  /// Evaluate the surface point at parameters `(u, v)`, each in `[0, 1]`.
  pub fn evaluate(&self, u: T, v: T) -> Vec3<T> {
    let row_count = self.v_count();

    // Evaluate each u-direction row at u, collect intermediate curve points
    let mut intermediate = Vec::with_capacity(row_count);
    for v_idx in 0..row_count {
      let start = v_idx * self.u_count();
      let row = &self.control_points[start..start + self.u_count()];
      intermediate.push(Self::de_casteljau_curve(row, u));
    }

    // Evaluate the v-direction column at v
    let result = Self::de_casteljau_curve(&intermediate, v);
    Vec3::new(result.x / result.w, result.y / result.w, result.z / result.w)
  }

  /// Evaluate surface point and first-order partial derivatives.
  ///
  /// Returns `(point, du, dv)` where `du = ∂S/∂u` and `dv = ∂S/∂v`.
  /// Uses the hodograph (forward difference) of control points together
  /// with the rational derivative formula.
  pub fn evaluate_partial(&self, u: T, v: T) -> (Vec3<T>, Vec3<T>, Vec3<T>) {
    let (a, au, av) = self.evaluate_homogeneous_derivatives(u, v);

    let w = a.w;
    let w2 = w * w;

    let point = Vec3::new(a.x / w, a.y / w, a.z / w);

    let su = Vec3::new(
      (au.x * w - a.x * au.w) / w2,
      (au.y * w - a.y * au.w) / w2,
      (au.z * w - a.z * au.w) / w2,
    );

    let sv = Vec3::new(
      (av.x * w - a.x * av.w) / w2,
      (av.y * w - a.y * av.w) / w2,
      (av.z * w - a.z * av.w) / w2,
    );

    (point, su, sv)
  }
}

// --- Private helpers ---

impl<T: Scalar> RationalBezierSurface<T> {
  /// Evaluate A, A_u, A_v in homogeneous coordinates, where:
  /// - `A` = weighted sum of control points = (w·S, w)
  /// - `A_u` = derivative of A with respect to u
  /// - `A_v` = derivative of A with respect to v
  fn evaluate_homogeneous_derivatives(
    &self,
    u: T,
    v: T,
  ) -> (Vec4<T>, Vec4<T>, Vec4<T>) {
    let row_count = self.v_count();
    let col_count = self.u_count();

    // A: full degree (m, n) surface
    let mut intermediate = Vec::with_capacity(row_count);
    for v_idx in 0..row_count {
      let start = v_idx * col_count;
      let row = &self.control_points[start..start + col_count];
      intermediate.push(Self::de_casteljau_curve(row, u));
    }
    let a = Self::de_casteljau_curve(&intermediate, v);

    // A_u: hodograph in u — forward differences, degree (m-1, n)
    let mut du_curves = Vec::with_capacity(row_count);
    for v_idx in 0..row_count {
      let start = v_idx * col_count;
      let du_row: Vec<Vec4<T>> = (0..self.u_degree)
        .map(|i| {
          self.control_points[start + i + 1] - self.control_points[start + i]
        })
        .collect();
      du_curves.push(Self::de_casteljau_curve(&du_row, u));
    }
    let au_raw = Self::de_casteljau_curve(&du_curves, v);
    let u_deg_t = T::from(self.u_degree).expect("degree should be representable");
    let au = Vec4::new(
      au_raw.x * u_deg_t,
      au_raw.y * u_deg_t,
      au_raw.z * u_deg_t,
      au_raw.w * u_deg_t,
    );

    // A_v: hodograph in v — forward differences, degree (m, n-1)
    let mut dv_rows = Vec::with_capacity(self.v_degree);
    for v_idx in 0..self.v_degree {
      let cp_row_start = v_idx * col_count;
      let dv_row: Vec<Vec4<T>> = (0..col_count)
        .map(|i| {
          self.control_points[(v_idx + 1) * col_count + i]
            - self.control_points[cp_row_start + i]
        })
        .collect();
      dv_rows.push(Self::de_casteljau_curve(&dv_row, u));
    }
    let av_raw = Self::de_casteljau_curve(&dv_rows, v);
    let v_deg_t = T::from(self.v_degree).expect("degree should be representable");
    let av = Vec4::new(
      av_raw.x * v_deg_t,
      av_raw.y * v_deg_t,
      av_raw.z * v_deg_t,
      av_raw.w * v_deg_t,
    );

    (a, au, av)
  }

  /// Evaluate a rational Bézier curve in homogeneous coordinates
  /// using the de Casteljau algorithm. Returns a `Vec4<T>` on the curve.
  fn de_casteljau_curve(points: &[Vec4<T>], t: T) -> Vec4<T> {
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

// --- ParametricSurface impl ---

impl ParametricSurface for RationalBezierSurface<f32> {
  fn position(&self, position: Vec2<f32>) -> Vec3<f32> {
    self.evaluate(position.x, position.y)
  }

  fn normal_dir(&self, position: Vec2<f32>) -> Vec3<f32> {
    let (_, du, dv) = self.evaluate_partial(position.x, position.y);
    du.cross(dv)
  }
}
