use rendiation_algebra::*;
use rendiation_mesh_generator::ParametricSurface;

use super::bezier::RationalBezierSurface;

/// A NURBS (Non-Uniform Rational B-Spline) surface of arbitrary degree.
///
/// The surface is defined by a grid of control points with weights (in homogeneous
/// coordinates), two knot vectors, and degrees for each parametric direction.
///
/// Control points are stored as `Vec4<T>` where `(x, y, z)` are the geometric
/// coordinates and `w` is the weight. Points are laid out in row-major order:
/// `index = v * u_count + u`.
#[derive(Debug, Clone)]
pub struct NurbsSurface<T> {
  control_points: Vec<Vec4<T>>,
  u_count: usize,
  v_count: usize,
  u_degree: usize,
  v_degree: usize,
  u_knots: Vec<T>,
  v_knots: Vec<T>,
}

impl<T: Scalar> NurbsSurface<T> {
  /// Create a new NURBS surface.
  ///
  /// * `control_points` - Weighted control points in row-major order (length = `u_count * v_count`).
  /// * `u_count` / `v_count` - Number of control points in each direction.
  /// * `u_degree` / `v_degree` - Degree in each direction (≥ 1).
  /// * `u_knots` / `v_knots` - Knot vectors (length = `count + degree + 1`).
  pub fn new(
    control_points: Vec<Vec4<T>>,
    u_count: usize,
    v_count: usize,
    u_degree: usize,
    v_degree: usize,
    u_knots: Vec<T>,
    v_knots: Vec<T>,
  ) -> Self {
    debug_assert_eq!(control_points.len(), u_count * v_count);
    debug_assert_eq!(u_knots.len(), u_count + u_degree + 1);
    debug_assert_eq!(v_knots.len(), v_count + v_degree + 1);
    debug_assert!(u_degree >= 1);
    debug_assert!(v_degree >= 1);
    Self {
      control_points,
      u_count,
      v_count,
      u_degree,
      v_degree,
      u_knots,
      v_knots,
    }
  }

  /// Create a NURBS surface from unweighted 3D control points (all weights = 1),
  /// making it a B-spline surface in homogeneous form.
  pub fn from_unweighted(
    points: Vec<Vec3<T>>,
    u_count: usize,
    v_count: usize,
    u_degree: usize,
    v_degree: usize,
    u_knots: Vec<T>,
    v_knots: Vec<T>,
  ) -> Self {
    let control_points: Vec<Vec4<T>> = points
      .into_iter()
      .map(|p| Vec4::new(p.x, p.y, p.z, T::one()))
      .collect();
    Self::new(
      control_points,
      u_count,
      v_count,
      u_degree,
      v_degree,
      u_knots,
      v_knots,
    )
  }

  // --- Accessors ---

  pub fn u_count(&self) -> usize {
    self.u_count
  }
  pub fn v_count(&self) -> usize {
    self.v_count
  }
  pub fn u_degree(&self) -> usize {
    self.u_degree
  }
  pub fn v_degree(&self) -> usize {
    self.v_degree
  }
  pub fn u_knots(&self) -> &[T] {
    &self.u_knots
  }
  pub fn v_knots(&self) -> &[T] {
    &self.v_knots
  }
  pub fn control_points(&self) -> &[Vec4<T>] {
    &self.control_points
  }

  pub fn control_point(&self, u_idx: usize, v_idx: usize) -> Vec4<T> {
    self.control_points[v_idx * self.u_count + u_idx]
  }

  /// Valid parameter range in u: `[u_knots[degree], u_knots[count]]`.
  pub fn u_range(&self) -> (T, T) {
    (self.u_knots[self.u_degree], self.u_knots[self.u_count])
  }

  /// Valid parameter range in v: `[v_knots[degree], v_knots[count]]`.
  pub fn v_range(&self) -> (T, T) {
    (self.v_knots[self.v_degree], self.v_knots[self.v_count])
  }

  // --- Evaluation ---

  /// Evaluate the surface point at parameters `(u, v)`.
  pub fn evaluate(&self, u: T, v: T) -> Vec3<T> {
    let u_span = Self::find_knot_span(self.u_count - 1, self.u_degree, u, &self.u_knots);
    let v_span = Self::find_knot_span(self.v_count - 1, self.v_degree, v, &self.v_knots);

    let nu = Self::basis_functions(u_span, u, self.u_degree, &self.u_knots);
    let nv = Self::basis_functions(v_span, v, self.v_degree, &self.v_knots);

    let mut sum = Vec4::splat(T::zero());
    for l in 0..=self.v_degree {
      let v_idx = v_span - self.v_degree + l;
      let nv_val = nv[l];
      for k in 0..=self.u_degree {
        let u_idx = u_span - self.u_degree + k;
        let basis = nu[k] * nv_val;
        let cp = self.control_points[v_idx * self.u_count + u_idx];
        sum.x += basis * cp.x;
        sum.y += basis * cp.y;
        sum.z += basis * cp.z;
        sum.w += basis * cp.w;
      }
    }

    Vec3::new(sum.x / sum.w, sum.y / sum.w, sum.z / sum.w)
  }

  /// Evaluate the surface point and first-order partial derivatives.
  ///
  /// Returns `(point, du, dv)` where `du = ∂S/∂u` and `dv = ∂S/∂v`.
  pub fn evaluate_partial(&self, u: T, v: T) -> (Vec3<T>, Vec3<T>, Vec3<T>) {
    let u_span = Self::find_knot_span(self.u_count - 1, self.u_degree, u, &self.u_knots);
    let v_span = Self::find_knot_span(self.v_count - 1, self.v_degree, v, &self.v_knots);

    let (nu, nu_deriv) =
      Self::basis_functions_and_derivatives(u_span, u, self.u_degree, &self.u_knots);
    let (nv, nv_deriv) =
      Self::basis_functions_and_derivatives(v_span, v, self.v_degree, &self.v_knots);

    let mut a = Vec4::splat(T::zero());
    let mut au = Vec4::splat(T::zero());
    let mut av = Vec4::splat(T::zero());

    for l in 0..=self.v_degree {
      let v_idx = v_span - self.v_degree + l;
      for k in 0..=self.u_degree {
        let u_idx = u_span - self.u_degree + k;
        let cp = self.control_points[v_idx * self.u_count + u_idx];

        let basis = nu[k] * nv[l];
        a.x += basis * cp.x;
        a.y += basis * cp.y;
        a.z += basis * cp.z;
        a.w += basis * cp.w;

        let u_basis = nu_deriv[k] * nv[l];
        au.x += u_basis * cp.x;
        au.y += u_basis * cp.y;
        au.z += u_basis * cp.z;
        au.w += u_basis * cp.w;

        let v_basis = nu[k] * nv_deriv[l];
        av.x += v_basis * cp.x;
        av.y += v_basis * cp.y;
        av.z += v_basis * cp.z;
        av.w += v_basis * cp.w;
      }
    }

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

  /// Decompose the NURBS surface into a grid of mathematically equivalent
  /// rational Bézier patches by inserting all interior knots to full multiplicity.
  ///
  /// Returns patches indexed as `[v_segment][u_segment]`. If the NURBS has no
  /// interior knots (already Bézier form), the result is a single `1×1` grid.
  pub fn to_bezier_patches(&self) -> Vec<Vec<RationalBezierSurface<T>>> {
    let pu = self.u_degree;
    let pv = self.v_degree;

    // Convert flat storage to 2D grid for easier row/column manipulation
    let mut grid: Vec<Vec<Vec4<T>>> = (0..self.v_count)
      .map(|v| {
        (0..self.u_count)
          .map(|u| self.control_points[v * self.u_count + u])
          .collect()
      })
      .collect();
    let mut u_knots = self.u_knots.clone();
    let mut v_knots = self.v_knots.clone();

    // Collect interior knots (distinct values with their current multiplicity)
    let u_info = Self::interior_knot_info(&u_knots, pu);
    let v_info = Self::interior_knot_info(&v_knots, pv);

    // Refine u direction
    for (knot, mult) in u_info {
      for _ in mult..pu {
        Self::insert_knot_u(&mut grid, &mut u_knots, pu, knot);
      }
    }

    // Refine v direction
    for (knot, mult) in v_info {
      for _ in mult..pv {
        Self::insert_knot_v(&mut grid, &mut v_knots, pv, knot);
      }
    }

    let u_cp = grid[0].len();
    let v_cp = grid.len();
    let u_seg = (u_cp - 1) / pu;
    let v_seg = (v_cp - 1) / pv;

    let mut patches = Vec::with_capacity(v_seg);
    for vs in 0..v_seg {
      let mut row = Vec::with_capacity(u_seg);
      for us in 0..u_seg {
        let u_start = us * pu;
        let v_start = vs * pv;
        let mut cp = Vec::with_capacity((pu + 1) * (pv + 1));
        for vi in v_start..=v_start + pv {
          for ui in u_start..=u_start + pu {
            cp.push(grid[vi][ui]);
          }
        }
        row.push(RationalBezierSurface::new(cp, pu, pv));
      }
      patches.push(row);
    }

    patches
  }
}

// --- Private helpers ---

impl<T: Scalar> NurbsSurface<T> {
  /// Find the knot span index i such that `knots[i] <= u < knots[i+1]`.
  /// `n` is the index of the last control point (`count - 1`), `p` is the degree.
  fn find_knot_span(n: usize, p: usize, u: T, knots: &[T]) -> usize {
    if u >= knots[n + 1] {
      return n;
    }
    if u <= knots[p] {
      return p;
    }

    let mut low = p;
    let mut high = n + 1;
    let mut mid = (low + high) >> 1;

    while u < knots[mid] || u >= knots[mid + 1] {
      if u < knots[mid] {
        high = mid;
      } else {
        low = mid;
      }
      mid = (low + high) >> 1;
    }

    mid
  }

  /// Compute all non-zero basis functions `N_{span-p+i, p}(u)` for `i = 0..=p`.
  /// Algorithm A2.2 from *The NURBS Book* (Piegl & Tiller).
  fn basis_functions(span: usize, u: T, p: usize, knots: &[T]) -> Vec<T> {
    let mut n = vec![T::zero(); p + 1];
    n[0] = T::one();

    let mut left = vec![T::zero(); p + 1];
    let mut right = vec![T::zero(); p + 1];

    for j in 1..=p {
      left[j] = u - knots[span + 1 - j];
      right[j] = knots[span + j] - u;
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

  /// Compute basis functions `N_{i,p}(u)` and their first derivatives `N'_{i,p}(u)`.
  ///
  /// Returns `(basis_values, derivative_values)`.
  fn basis_functions_and_derivatives(span: usize, u: T, p: usize, knots: &[T]) -> (Vec<T>, Vec<T>) {
    let n = Self::basis_functions(span, u, p, knots);

    if p == 0 {
      return (n, vec![T::zero(); 1]);
    }

    // Derivatives via p-1 degree basis: N'(k,p) = p/(u_{k+p}-u_k)*N(k,p-1) - p/(u_{k+p+1}-u_{k+1})*N(k+1,p-1)
    let n_low = Self::basis_functions(span, u, p - 1, knots);
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
  /// Interior knots are `knots[degree + 1 .. len - degree - 1]`.
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

  /// Insert a knot `u` into the u direction (each row is a curve).
  /// Algorithm A5.1 from *The NURBS Book* (Piegl & Tiller).
  fn insert_knot_u(grid: &mut Vec<Vec<Vec4<T>>>, knots: &mut Vec<T>, degree: usize, u: T) {
    let n = grid[0].len() - 1;
    let k = Self::find_knot_span(n, degree, u, knots);

    // Compute α_i = (u - u_i) / (u_{i+degree} - u_i) for i = k-degree+1 .. k
    let start = (k + 1).saturating_sub(degree);
    let mut alphas = vec![T::zero(); k + 1];
    for i in start..=k {
      let denom = knots[i + degree] - knots[i];
      alphas[i] = if denom > T::zero() {
        (u - knots[i]) / denom
      } else {
        T::zero()
      };
    }

    knots.insert(k + 1, u);

    let old_len = n + 1;
    for row in grid.iter_mut() {
      let mut new_cp = Vec::with_capacity(old_len + 1);

      // Unchanged: indices 0 .. k-degree+1
      for i in 0..start {
        new_cp.push(row[i]);
      }
      // Modified: indices k-degree+1 .. k
      for i in start..=k {
        let alpha = alphas[i];
        new_cp.push(row[i] * alpha + row[i - 1] * (T::one() - alpha));
      }
      // Shifted: indices k+1 .. n+1, where Q_i = P_{i-1}
      for i in k + 1..=old_len {
        new_cp.push(row[i - 1]);
      }

      *row = new_cp;
    }
  }

  /// Insert a knot `v` into the v direction (each column is a curve).
  /// Follows the same formula as `insert_knot_u`, transposed.
  fn insert_knot_v(grid: &mut Vec<Vec<Vec4<T>>>, knots: &mut Vec<T>, degree: usize, v: T) {
    let n = grid.len() - 1;
    let k = Self::find_knot_span(n, degree, v, knots);

    let start = (k + 1).saturating_sub(degree);
    let mut alphas = vec![T::zero(); k + 1];
    for i in start..=k {
      let denom = knots[i + degree] - knots[i];
      alphas[i] = if denom > T::zero() {
        (v - knots[i]) / denom
      } else {
        T::zero()
      };
    }

    knots.insert(k + 1, v);

    let old_len = n + 1;
    let u_count = grid[0].len();
    let mut new_grid = Vec::with_capacity(old_len + 1);

    // Unchanged: rows 0 .. k-degree+1
    for i in 0..start {
      new_grid.push(grid[i].clone());
    }
    // Modified: rows k-degree+1 .. k
    for i in start..=k {
      let alpha = alphas[i];
      let one_minus_alpha = T::one() - alpha;
      let new_row: Vec<Vec4<T>> = (0..u_count)
        .map(|col| grid[i][col] * alpha + grid[i - 1][col] * one_minus_alpha)
        .collect();
      new_grid.push(new_row);
    }
    // Shifted: rows k+1 .. n+1, where Q_i = P_{i-1}
    for i in k + 1..=old_len {
      new_grid.push(grid[i - 1].clone());
    }

    *grid = new_grid;
  }
}

// --- ParametricSurface impl ---

impl ParametricSurface for NurbsSurface<f32> {
  fn position(&self, uv: Vec2<f32>) -> Vec3<f32> {
    let (u_min, u_max) = self.u_range();
    let (v_min, v_max) = self.v_range();
    let u = u_min + uv.x * (u_max - u_min);
    let v = v_min + uv.y * (v_max - v_min);
    self.evaluate(u, v)
  }

  fn normal_dir(&self, uv: Vec2<f32>) -> Vec3<f32> {
    let (u_min, u_max) = self.u_range();
    let (v_min, v_max) = self.v_range();
    let u = u_min + uv.x * (u_max - u_min);
    let v = v_min + uv.y * (v_max - v_min);
    let (_, du, dv) = self.evaluate_partial(u, v);
    du.cross(dv)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn points_close(a: Vec3<f32>, b: Vec3<f32>, eps: f32) -> bool {
    (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
  }

  /// A Bézier-form NURBS (no interior knots) should convert to exactly 1 patch
  /// that evaluates identically.
  #[test]
  fn bezier_form_nurbs_to_single_patch() {
    // 4×4 control net, degree 3, clamped knots → Bézier form
    let points: Vec<Vec3<f32>> = (0..4)
      .flat_map(|v| (0..4).map(move |u| Vec3::new(u as f32, v as f32, 0.)))
      .collect();
    let knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    let nurbs = NurbsSurface::from_unweighted(points, 4, 4, 3, 3, knots.clone(), knots);

    let patches = nurbs.to_bezier_patches();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].len(), 1);

    // Check point equality at sample UVs
    let patch = &patches[0][0];
    for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
      for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let n_pt = nurbs.evaluate(u, v);
        let b_pt = patch.evaluate(u, v);
        assert!(
          points_close(n_pt, b_pt, 1e-5),
          "mismatch at ({u}, {v}): NURBS {n_pt:?} vs Bézier {b_pt:?}"
        );
      }
    }
  }

  /// A NURBS with interior knots should decompose into multiple patches
  /// that match the original surface in their respective parameter sub-ranges.
  #[test]
  fn nurbs_with_interior_knots_decomposition() {
    // 4×4 control net, degree 2, one interior knot at 0.5 in both dirs
    // → refined gives 2×2 = 4 patches
    let points: Vec<Vec3<f32>> = (0..4)
      .flat_map(|v| {
        (0..4).map(move |u| {
          let x = u as f32 - 1.5;
          let y = v as f32 - 1.5;
          Vec3::new(x, y, (x * 0.5).sin() * (y * 0.5).cos())
        })
      })
      .collect();
    // degree 2, 4 CPs → 7 knots
    let knots = vec![0., 0., 0., 0.5, 1., 1., 1.];
    let nurbs = NurbsSurface::from_unweighted(points, 4, 4, 2, 2, knots.clone(), knots);

    let patches = nurbs.to_bezier_patches();
    assert_eq!(patches.len(), 2, "v segment count");
    assert_eq!(patches[0].len(), 2, "u segment count");

    // For each patch, verify that evaluating at its local [0,1]² matches the
    // NURBS at the corresponding global parameter sub-range.
    let (u_min, u_max) = nurbs.u_range();
    let (v_min, v_max) = nurbs.v_range();
    let u_seg = patches[0].len();
    let v_seg = patches.len();

    for vs in 0..v_seg {
      for us in 0..u_seg {
        let patch = &patches[vs][us];
        let u0 = u_min + (u_max - u_min) * (us as f32 / u_seg as f32);
        let u1 = u_min + (u_max - u_min) * ((us + 1) as f32 / u_seg as f32);
        let v0 = v_min + (v_max - v_min) * (vs as f32 / v_seg as f32);
        let v1 = v_min + (v_max - v_min) * ((vs + 1) as f32 / v_seg as f32);

        for t in [0.1, 0.5, 0.9] {
          let gu = u0 + t * (u1 - u0);
          let gv = v0 + t * (v1 - v0);
          let n_pt = nurbs.evaluate(gu, gv);
          let lu = (gu - u0) / (u1 - u0);
          let lv = (gv - v0) / (v1 - v0);
          let b_pt = patch.evaluate(lu, lv);

          assert!(
            points_close(n_pt, b_pt, 1e-4),
            "patch ({us},{vs}) at local ({lu:.3},{lv:.3}) → global ({gu:.3},{gv:.3}): \
             NURBS {n_pt:?} vs Bézier {b_pt:?}"
          );
        }
      }
    }

    // Continuity check: adjacent patches should share boundary points.
    // u-seam: left patch (us, vs) u=1 edge ≡ right patch (us+1, vs) u=0 edge
    for vs in 0..v_seg {
      for us in 0..u_seg - 1 {
        let left = &patches[vs][us];
        let right = &patches[vs][us + 1];
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
          let lp = left.evaluate(1.0, t);
          let rp = right.evaluate(0.0, t);
          assert!(
            points_close(lp, rp, 1e-5),
            "u-seam discontinuity at v={t}: left {lp:?} vs right {rp:?}"
          );
        }
      }
    }
    // v-seam: bottom patch (us, vs) v=1 edge ≡ top patch (us, vs+1) v=0 edge
    for vs in 0..v_seg - 1 {
      for us in 0..u_seg {
        let bottom = &patches[vs][us];
        let top = &patches[vs + 1][us];
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
          let bp = bottom.evaluate(t, 1.0);
          let tp = top.evaluate(t, 0.0);
          assert!(
            points_close(bp, tp, 1e-5),
            "v-seam discontinuity at u={t}: bottom {bp:?} vs top {tp:?}"
          );
        }
      }
    }
  }
}
