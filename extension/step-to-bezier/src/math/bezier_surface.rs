use rendiation_algebra::*;

use super::*;

/// Tensor product rational Bézier patch over `[0,1]²`. Control point layout is
/// `control_points[v * (degree_u + 1) + u]`, homogeneous `(x·w, y·w, z·w, w)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RationalBezierSurface {
  pub degree_u: usize,
  pub degree_v: usize,
  pub control_points: Vec<Vec4<f64>>,
}

impl RationalBezierSurface {
  pub fn new(degree_u: usize, degree_v: usize, control_points: Vec<Vec4<f64>>) -> Self {
    assert_eq!(control_points.len(), (degree_u + 1) * (degree_v + 1));
    Self {
      degree_u,
      degree_v,
      control_points,
    }
  }

  pub fn control_point(&self, u: usize, v: usize) -> Vec4<f64> {
    self.control_points[v * (self.degree_u + 1) + u]
  }

  pub fn eval_homogeneous(&self, u: f64, v: f64) -> Vec4<f64> {
    let column: Vec<_> = self.rows().map(|row| de_casteljau(row, u)).collect();
    de_casteljau(&column, v)
  }

  pub fn eval(&self, u: f64, v: f64) -> Vec3<f64> {
    self.eval_homogeneous(u, v).to_euclid()
  }

  /// Point, ∂S/∂u, ∂S/∂v.
  pub fn eval_with_partials(&self, u: f64, v: f64) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (column, column_du): (Vec<_>, Vec<_>) = self
      .rows()
      .map(|row| de_casteljau_with_derivative(row, u))
      .unzip();
    let (h, h_v) = de_casteljau_with_derivative(&column, v);
    let h_u = de_casteljau(&column_du, v);
    let (p, du) = rational_first_derivative(h, h_u);
    let (_, dv) = rational_first_derivative(h, h_v);
    (p, du, dv)
  }

  /// Isoparametric curve at constant `axis` coordinate `value`.
  pub fn iso_curve(&self, axis: Axis, value: f64) -> RationalBezierCurve3 {
    match axis {
      Axis::U => {
        RationalBezierCurve::new(self.rows().map(|row| de_casteljau(row, value)).collect())
      }
      Axis::V => RationalBezierCurve::new(
        (0..=self.degree_u)
          .map(|i| de_casteljau(&self.column(i), value))
          .collect(),
      ),
    }
  }

  /// Sub patch over `[u0,u1] × [v0,v1]` reparameterized to `[0,1]²`. Reversed ranges flip the
  /// corresponding direction.
  pub fn sub_patch(&self, u: (f64, f64), v: (f64, f64)) -> Self {
    let nu = self.degree_u + 1;
    let mut cp = Vec::with_capacity(self.control_points.len());
    for row in self.rows() {
      cp.extend(
        RationalBezierCurve::new(row.to_vec())
          .sub_curve(u.0, u.1)
          .control_points,
      );
    }
    for i in 0..nu {
      let column: Vec<_> = (0..=self.degree_v).map(|j| cp[j * nu + i]).collect();
      let sub = RationalBezierCurve::new(column).sub_curve(v.0, v.1);
      for (j, p) in sub.control_points.into_iter().enumerate() {
        cp[j * nu + i] = p;
      }
    }
    Self::new(self.degree_u, self.degree_v, cp)
  }

  pub fn transform(&self, mat: Mat4<f64>) -> Self {
    let control_points = self.control_points.iter().map(|p| mat * *p).collect();
    Self::new(self.degree_u, self.degree_v, control_points)
  }

  fn rows(&self) -> impl Iterator<Item = &[Vec4<f64>]> {
    self.control_points.chunks_exact(self.degree_u + 1)
  }

  fn column(&self, i: usize) -> Vec<Vec4<f64>> {
    self.rows().map(|row| row[i]).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Biquadratic x biquadratic-ish test patch: quarter cylinder in u (rational), cubic in v.
  fn test_patch() -> RationalBezierSurface {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let mut cp = Vec::new();
    for j in 0..4 {
      let z = j as f64;
      let r = 1. + 0.3 * (j as f64 - 1.5).powi(2);
      let pts = [
        (Vec3::new(r, 0., z), 1.),
        (Vec3::new(r, r, z + 0.2), h),
        (Vec3::new(0., r, z), 1.),
      ];
      for (p, w) in pts {
        cp.push(Vec4::from_euclid(p, w * (1. + 0.1 * j as f64)));
      }
    }
    RationalBezierSurface::new(2, 3, cp)
  }

  fn bernstein(n: usize, i: usize, t: f64) -> f64 {
    let binom = (0..i).fold(1., |b, k| b * (n - k) as f64 / (k + 1) as f64);
    binom * t.powi(i as i32) * (1. - t).powi((n - i) as i32)
  }

  fn direct_eval(s: &RationalBezierSurface, u: f64, v: f64) -> Vec3<f64> {
    let mut acc = Vec4::new(0., 0., 0., 0.);
    for j in 0..=s.degree_v {
      for i in 0..=s.degree_u {
        acc += s.control_point(i, j) * (bernstein(s.degree_u, i, u) * bernstein(s.degree_v, j, v));
      }
    }
    acc.to_euclid()
  }

  fn d(a: Vec3<f64>, b: Vec3<f64>) -> f64 {
    (a - b).length()
  }

  #[test]
  fn eval_and_partials() {
    let s = test_patch();
    let h = 1e-6;
    for i in 0..=8 {
      for j in 0..=8 {
        let (u, v) = (i as f64 / 8., j as f64 / 8.);
        let (p, du, dv) = s.eval_with_partials(u, v);
        assert!(d(p, direct_eval(&s, u, v)) < 1e-13);
        assert!(d(s.eval(u, v), p) < 1e-14);
        let fu = (s.eval(u + h, v) - s.eval(u - h, v)) * (0.5 / h);
        let fv = (s.eval(u, v + h) - s.eval(u, v - h)) * (0.5 / h);
        assert!(d(du, fu) < 1e-6 * (1. + du.length()));
        assert!(d(dv, fv) < 1e-6 * (1. + dv.length()));
      }
    }
  }

  #[test]
  fn iso_curves() {
    let s = test_patch();
    for k in 0..=10 {
      let c = k as f64 / 10.;
      let cu = s.iso_curve(Axis::U, c);
      let cv = s.iso_curve(Axis::V, c);
      assert_eq!(cu.degree(), s.degree_v);
      assert_eq!(cv.degree(), s.degree_u);
      for i in 0..=10 {
        let t = i as f64 / 10.;
        assert!(d(cu.eval(t), s.eval(c, t)) < 1e-13);
        assert!(d(cv.eval(t), s.eval(t, c)) < 1e-13);
      }
    }
  }

  #[test]
  fn sub_patch_matches() {
    let s = test_patch();
    let sub = s.sub_patch((0.2, 0.9), (0.75, 0.1));
    for i in 0..=10 {
      for j in 0..=10 {
        let (a, b) = (i as f64 / 10., j as f64 / 10.);
        let expected = s.eval(0.2 + 0.7 * a, 0.75 - 0.65 * b);
        assert!(d(sub.eval(a, b), expected) < 1e-13);
      }
    }
  }

  #[test]
  fn transform_matches() {
    let s = test_patch();
    let mat = Mat4::translate((0., 1., -2.)) * Mat4::rotate_x(1.1);
    let t = s.transform(mat);
    for i in 0..=5 {
      for j in 0..=5 {
        let (a, b) = (i as f64 / 5., j as f64 / 5.);
        let p = s.eval(a, b);
        let expected = (mat * Vec4::new(p.x, p.y, p.z, 1.)).to_euclid();
        assert!(d(t.eval(a, b), expected) < 1e-12);
      }
    }
  }
}
