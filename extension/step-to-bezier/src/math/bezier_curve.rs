use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

use rendiation_algebra::*;

/// A point in homogeneous coordinates `(p·w, w)`.
pub trait HomogeneousPoint:
  Copy + Debug + Add<Output = Self> + Sub<Output = Self> + Mul<f64, Output = Self>
{
  type Euclid: Copy
    + Debug
    + Add<Output = Self::Euclid>
    + Sub<Output = Self::Euclid>
    + Mul<f64, Output = Self::Euclid>;
  fn weight(self) -> f64;
  /// Perspective divide.
  fn to_euclid(self) -> Self::Euclid;
  fn from_euclid(p: Self::Euclid, w: f64) -> Self;
  fn euclid_distance(a: Self::Euclid, b: Self::Euclid) -> f64;
  /// The non weight components `p·w` as is, without the perspective divide. Used for
  /// derivatives where the homogeneous weight may be zero.
  fn vector_part(self) -> Self::Euclid;
  fn euclid_dot(a: Self::Euclid, b: Self::Euclid) -> f64;
}

impl HomogeneousPoint for Vec3<f64> {
  type Euclid = Vec2<f64>;
  fn weight(self) -> f64 {
    self.z
  }
  fn to_euclid(self) -> Vec2<f64> {
    Vec2::new(self.x / self.z, self.y / self.z)
  }
  fn from_euclid(p: Vec2<f64>, w: f64) -> Self {
    Vec3::new(p.x * w, p.y * w, w)
  }
  fn euclid_distance(a: Vec2<f64>, b: Vec2<f64>) -> f64 {
    (a - b).length()
  }
  fn vector_part(self) -> Vec2<f64> {
    Vec2::new(self.x, self.y)
  }
  fn euclid_dot(a: Vec2<f64>, b: Vec2<f64>) -> f64 {
    a.dot(b)
  }
}

impl HomogeneousPoint for Vec4<f64> {
  type Euclid = Vec3<f64>;
  fn weight(self) -> f64 {
    self.w
  }
  fn to_euclid(self) -> Vec3<f64> {
    Vec3::new(self.x / self.w, self.y / self.w, self.z / self.w)
  }
  fn from_euclid(p: Vec3<f64>, w: f64) -> Self {
    Vec4::new(p.x * w, p.y * w, p.z * w, w)
  }
  fn euclid_distance(a: Vec3<f64>, b: Vec3<f64>) -> f64 {
    (a - b).length()
  }
  fn vector_part(self) -> Vec3<f64> {
    Vec3::new(self.x, self.y, self.z)
  }
  fn euclid_dot(a: Vec3<f64>, b: Vec3<f64>) -> f64 {
    a.dot(b)
  }
}

/// Rational Bézier curve over `t ∈ [0, 1]` with homogeneous control points. Degree is
/// `control_points.len() - 1`. All weights are expected to be positive.
#[derive(Debug, Clone, PartialEq)]
pub struct RationalBezierCurve<P: HomogeneousPoint> {
  pub control_points: Vec<P>,
}

pub type RationalBezierCurve2 = RationalBezierCurve<Vec3<f64>>;
pub type RationalBezierCurve3 = RationalBezierCurve<Vec4<f64>>;

impl<P: HomogeneousPoint> RationalBezierCurve<P> {
  pub fn new(control_points: Vec<P>) -> Self {
    Self { control_points }
  }

  /// `weights`, when given, must have the same length as `points`.
  pub fn from_euclid(points: &[P::Euclid], weights: Option<&[f64]>) -> Self {
    if let Some(w) = weights {
      assert_eq!(w.len(), points.len(), "weight count mismatch");
    }
    let control_points = points
      .iter()
      .enumerate()
      .map(|(i, p)| P::from_euclid(*p, weights.map_or(1., |w| w[i])))
      .collect();
    Self { control_points }
  }

  /// Straight line segment (degree 1, unit weights).
  pub fn line(a: P::Euclid, b: P::Euclid) -> Self {
    Self::new(vec![P::from_euclid(a, 1.), P::from_euclid(b, 1.)])
  }

  pub fn degree(&self) -> usize {
    self.control_points.len() - 1
  }

  pub fn start(&self) -> P::Euclid {
    self.control_points[0].to_euclid()
  }

  pub fn end(&self) -> P::Euclid {
    self.control_points[self.control_points.len() - 1].to_euclid()
  }

  pub fn eval_homogeneous(&self, t: f64) -> P {
    de_casteljau(&self.control_points, t)
  }

  pub fn eval(&self, t: f64) -> P::Euclid {
    self.eval_homogeneous(t).to_euclid()
  }

  /// Point and first derivative (Euclidean, with respect to `t`).
  pub fn eval_with_derivative(&self, t: f64) -> (P::Euclid, P::Euclid) {
    let (h, dh) = de_casteljau_with_derivative(&self.control_points, t);
    rational_first_derivative(h, dh)
  }

  /// Point, first and second derivative (Euclidean, with respect to `t`).
  pub fn eval_with_second_derivative(&self, t: f64) -> (P::Euclid, P::Euclid, P::Euclid) {
    let n = self.degree();
    let h = self.eval_homogeneous(t);
    if n == 0 {
      let zero = h.vector_part() * 0.;
      return (h.to_euclid(), zero, zero);
    }
    let d1 = hodograph(&self.control_points);
    let dh = de_casteljau(&d1, t);
    let ddh = if n >= 2 {
      de_casteljau(&hodograph(&d1), t)
    } else {
      dh * 0.
    };
    let w = h.weight();
    let p = h.to_euclid();
    let d = (dh.vector_part() - p * dh.weight()) * (1. / w);
    let dd = (ddh.vector_part() - d * (2. * dh.weight()) - p * ddh.weight()) * (1. / w);
    (p, d, dd)
  }

  /// de Casteljau split at `t`.
  pub fn split(&self, t: f64) -> (Self, Self) {
    let n = self.control_points.len();
    let mut work = self.control_points.clone();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    left.push(work[0]);
    right.push(work[n - 1]);
    for k in 1..n {
      for i in 0..n - k {
        work[i] = blend(work[i], work[i + 1], t);
      }
      left.push(work[0]);
      right.push(work[n - 1 - k]);
    }
    right.reverse();
    (Self::new(left), Self::new(right))
  }

  /// The part over `[t0, t1]` reparameterized to `[0, 1]`. `t0 > t1` yields a reversed curve.
  pub fn sub_curve(&self, t0: f64, t1: f64) -> Self {
    if t0 > t1 {
      return self.sub_curve(t1, t0).reversed();
    }
    if t0 == t1 {
      let p = self.eval_homogeneous(t0);
      return Self::new(vec![p; self.control_points.len()]);
    }
    if t0 == 0. && t1 == 1. {
      return self.clone();
    }
    if t0 == 0. {
      return self.split(t1).0;
    }
    if t1 == 1. {
      return self.split(t0).1;
    }
    // Split at the parameter farther from 0 first, then cut the head off the left part.
    let (left, _) = self.split(t1);
    let (_, mid) = left.split(t0 / t1);
    mid
  }

  pub fn reversed(&self) -> Self {
    let mut control_points = self.control_points.clone();
    control_points.reverse();
    Self::new(control_points)
  }

  pub fn elevate_degree(&self) -> Self {
    let cp = &self.control_points;
    let n = cp.len() - 1;
    let m = (n + 1) as f64;
    let mut out = Vec::with_capacity(n + 2);
    out.push(cp[0]);
    for i in 1..=n {
      let a = i as f64 / m;
      out.push(cp[i - 1] * a + cp[i] * (1. - a));
    }
    out.push(cp[n]);
    Self::new(out)
  }

  /// Möbius reparameterization making both end weights 1 (same geometry).
  ///
  /// With `c = (w0 / wn)^(1/n)`, the homogeneous points become `P_i · c^i / w0`. The Euclidean
  /// control points are unchanged, only the parameterization changes.
  pub fn normalize_end_weights(&self) -> Self {
    let cp = &self.control_points;
    let n = cp.len() - 1;
    let w0 = cp[0].weight();
    if n == 0 {
      return Self::new(vec![cp[0] * (1. / w0)]);
    }
    let wn = cp[n].weight();
    let c = (w0 / wn).powf(1. / n as f64);
    let mut scale = 1. / w0;
    let mut out = Vec::with_capacity(n + 1);
    for (i, p) in cp.iter().enumerate() {
      if i == 0 || i == n {
        // Exact unit end weights.
        out.push(P::from_euclid(p.to_euclid(), 1.));
      } else {
        out.push(*p * scale);
      }
      scale *= c;
    }
    Self::new(out)
  }

  /// All weights equal (relative tolerance 1e-12), so the curve is a polynomial one.
  pub fn is_polynomial(&self) -> bool {
    let w0 = self.control_points[0].weight();
    self
      .control_points
      .iter()
      .all(|p| (p.weight() - w0).abs() <= 1e-12 * w0.abs())
  }

  /// Replace the Euclidean start / end point while keeping the end weights. Used to snap
  /// endpoints to shared vertices exactly.
  pub fn with_endpoints(&self, start: P::Euclid, end: P::Euclid) -> Self {
    let mut control_points = self.control_points.clone();
    let n = control_points.len() - 1;
    control_points[0] = P::from_euclid(start, control_points[0].weight());
    control_points[n] = P::from_euclid(end, control_points[n].weight());
    Self::new(control_points)
  }
}

impl RationalBezierCurve2 {
  /// Apply `p' = p ⊙ scale + offset` per axis (exact for rational curves).
  pub fn affine_map(&self, scale: Vec2<f64>, offset: Vec2<f64>) -> Self {
    let control_points = self
      .control_points
      .iter()
      .map(|p| {
        Vec3::new(
          p.x * scale.x + offset.x * p.z,
          p.y * scale.y + offset.y * p.z,
          p.z,
        )
      })
      .collect();
    Self::new(control_points)
  }

  pub fn translate(&self, offset: Vec2<f64>) -> Self {
    self.affine_map(Vec2::new(1., 1.), offset)
  }

  /// Bounding box of the Euclidean control points (contains the curve, weights positive).
  pub fn control_bbox(&self) -> (Vec2<f64>, Vec2<f64>) {
    let first = self.start();
    self
      .control_points
      .iter()
      .map(|p| p.to_euclid())
      .fold((first, first), |(min, max), p| {
        (
          Vec2::new(min.x.min(p.x), min.y.min(p.y)),
          Vec2::new(max.x.max(p.x), max.y.max(p.y)),
        )
      })
  }
}

impl RationalBezierCurve3 {
  /// Apply `mat` to the homogeneous control points (exact, also for projective matrices).
  pub fn transform(&self, mat: Mat4<f64>) -> Self {
    Self::new(self.control_points.iter().map(|p| mat * *p).collect())
  }
}

/// Values that can be affinely combined: homogeneous points, Euclidean vectors, scalars.
pub trait Blend: Copy + Add<Output = Self> + Sub<Output = Self> + Mul<f64, Output = Self> {}
impl<T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f64, Output = T>> Blend for T {}

/// `a·(1-t) + b·t`, exact at `t = 0` and `t = 1`.
#[inline]
pub fn blend<T: Blend>(a: T, b: T, t: f64) -> T {
  a * (1. - t) + b * t
}

const STACK_BUFFER: usize = 16;

fn with_buffer<T: Blend, R>(cp: &[T], f: impl FnOnce(&mut [T]) -> R) -> R {
  let n = cp.len();
  if n <= STACK_BUFFER {
    let mut buf = [cp[0]; STACK_BUFFER];
    buf[..n].copy_from_slice(cp);
    f(&mut buf[..n])
  } else {
    let mut buf = cp.to_vec();
    f(&mut buf)
  }
}

/// Evaluate a Bernstein polynomial with coefficients `cp` (non empty) at `t`.
pub fn de_casteljau<T: Blend>(cp: &[T], t: f64) -> T {
  with_buffer(cp, |buf| {
    let n = buf.len();
    for k in 1..n {
      for i in 0..n - k {
        buf[i] = blend(buf[i], buf[i + 1], t);
      }
    }
    buf[0]
  })
}

/// Value and first derivative of a Bernstein polynomial with coefficients `cp` (non empty).
pub fn de_casteljau_with_derivative<T: Blend>(cp: &[T], t: f64) -> (T, T) {
  with_buffer(cp, |buf| {
    let n = buf.len();
    if n == 1 {
      return (buf[0], buf[0] * 0.);
    }
    for k in 1..n - 1 {
      for i in 0..n - k {
        buf[i] = blend(buf[i], buf[i + 1], t);
      }
    }
    let (a, b) = (buf[0], buf[1]);
    (blend(a, b, t), (b - a) * (n - 1) as f64)
  })
}

/// Control points of the derivative (hodograph) of a Bernstein polynomial, degree `n - 1`.
pub fn hodograph<T: Blend>(cp: &[T]) -> Vec<T> {
  let n = (cp.len() - 1) as f64;
  cp.windows(2).map(|w| (w[1] - w[0]) * n).collect()
}

/// Quotient rule: Euclidean point and derivative from homogeneous point `h` and derivative `dh`.
pub fn rational_first_derivative<P: HomogeneousPoint>(h: P, dh: P) -> (P::Euclid, P::Euclid) {
  let p = h.to_euclid();
  let d = (dh.vector_part() - p * dh.weight()) * (1. / h.weight());
  (p, d)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn quarter_circle() -> RationalBezierCurve2 {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    RationalBezierCurve2::from_euclid(
      &[Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)],
      Some(&[1., h, 1.]),
    )
  }

  fn rational_cubic3() -> RationalBezierCurve3 {
    RationalBezierCurve3::from_euclid(
      &[
        Vec3::new(0., 0., 0.),
        Vec3::new(1., 2., 0.5),
        Vec3::new(3., -1., 1.),
        Vec3::new(4., 1., -2.),
      ],
      Some(&[1., 2.5, 0.4, 1.7]),
    )
  }

  fn dist2(a: Vec2<f64>, b: Vec2<f64>) -> f64 {
    (a - b).length()
  }

  fn dist3(a: Vec3<f64>, b: Vec3<f64>) -> f64 {
    (a - b).length()
  }

  /// Direct Bernstein sum evaluation for cross checking.
  fn bernstein_eval(c: &RationalBezierCurve3, t: f64) -> Vec3<f64> {
    let n = c.degree();
    let mut acc = Vec4::new(0., 0., 0., 0.);
    for (i, p) in c.control_points.iter().enumerate() {
      let binom = (0..i).fold(1., |b, k| b * (n - k) as f64 / (k + 1) as f64);
      let b = binom * t.powi(i as i32) * (1. - t).powi((n - i) as i32);
      acc += *p * b;
    }
    acc.to_euclid()
  }

  #[test]
  fn eval_matches_bernstein_sum() {
    let c = rational_cubic3();
    for i in 0..=50 {
      let t = i as f64 / 50.;
      assert!(dist3(c.eval(t), bernstein_eval(&c, t)) < 1e-13);
    }
    assert_eq!(c.start(), Vec3::new(0., 0., 0.));
    assert!(dist3(c.end(), Vec3::new(4., 1., -2.)) < 1e-15);
  }

  #[test]
  fn circle_is_exact() {
    let c = quarter_circle();
    for i in 0..=100 {
      let p = c.eval(i as f64 / 100.);
      assert!((p.length() - 1.).abs() < 1e-14);
    }
  }

  #[test]
  fn derivative_matches_finite_difference() {
    let c = rational_cubic3();
    let h = 1e-6;
    for i in 1..20 {
      let t = i as f64 / 20.;
      let (p, d) = c.eval_with_derivative(t);
      let fd = (c.eval(t + h) - c.eval(t - h)) * (0.5 / h);
      assert!(dist3(p, c.eval(t)) < 1e-14);
      assert!(dist3(d, fd) < 1e-6 * (1. + d.length()));

      let (p2, d2, dd) = c.eval_with_second_derivative(t);
      assert!(dist3(p2, p) < 1e-14);
      assert!(dist3(d2, d) < 1e-10 * (1. + d.length()));
      let fdd = (c.eval_with_derivative(t + h).1 - c.eval_with_derivative(t - h).1) * (0.5 / h);
      assert!(dist3(dd, fdd) < 1e-5 * (1. + dd.length()));
    }
    let line = RationalBezierCurve2::line(Vec2::new(1., 1.), Vec2::new(3., 2.));
    let (_, d, dd) = line.eval_with_second_derivative(0.3);
    assert!(dist2(d, Vec2::new(2., 1.)) < 1e-15);
    assert!(dd.length() < 1e-15);
    let point = RationalBezierCurve2::new(vec![Vec3::new(2., 2., 2.)]);
    assert_eq!(
      point.eval_with_derivative(0.5),
      (Vec2::new(1., 1.), Vec2::new(0., 0.))
    );
  }

  #[test]
  fn split_and_sub_curve() {
    let c = rational_cubic3();
    let (l, r) = c.split(0.3);
    for i in 0..=20 {
      let s = i as f64 / 20.;
      assert!(dist3(l.eval(s), c.eval(0.3 * s)) < 1e-13);
      assert!(dist3(r.eval(s), c.eval(0.3 + 0.7 * s)) < 1e-13);
    }
    let sub = c.sub_curve(0.2, 0.7);
    let rev = c.sub_curve(0.7, 0.2);
    for i in 0..=20 {
      let s = i as f64 / 20.;
      assert!(dist3(sub.eval(s), c.eval(0.2 + 0.5 * s)) < 1e-13);
      assert!(dist3(rev.eval(s), c.eval(0.7 - 0.5 * s)) < 1e-13);
    }
    assert_eq!(c.sub_curve(0., 1.), c);
    let degenerate = c.sub_curve(0.4, 0.4);
    assert!(dist3(degenerate.eval(0.9), c.eval(0.4)) < 1e-14);
    let reversed = c.sub_curve(1., 0.);
    assert_eq!(reversed, c.reversed());
  }

  #[test]
  fn elevate_keeps_geometry() {
    let c = rational_cubic3();
    let e = c.elevate_degree();
    assert_eq!(e.degree(), 4);
    for i in 0..=20 {
      let t = i as f64 / 20.;
      assert!(dist3(e.eval(t), c.eval(t)) < 1e-13);
    }
  }

  #[test]
  fn normalize_end_weights_keeps_geometry() {
    let c = rational_cubic3();
    let n = c.normalize_end_weights();
    assert_eq!(n.control_points[0].w, 1.);
    assert_eq!(n.control_points[3].w, 1.);
    assert_eq!(n.start(), c.start());
    assert!(dist3(n.end(), c.end()) < 1e-15);
    let w0 = c.control_points[0].w;
    let wn = c.control_points[3].w;
    let cc = (w0 / wn).powf(1. / 3.);
    for i in 0..=20 {
      let s = i as f64 / 20.;
      // The Möbius map relating both parameters.
      let t = cc * s / ((1. - s) + cc * s);
      assert!(dist3(n.eval(s), c.eval(t)) < 1e-13);
    }
    let q = quarter_circle().normalize_end_weights();
    for i in 0..=20 {
      assert!((q.eval(i as f64 / 20.).length() - 1.).abs() < 1e-14);
    }
  }

  #[test]
  fn polynomial_detection_and_endpoints() {
    assert!(!quarter_circle().is_polynomial());
    let poly = RationalBezierCurve2::new(vec![Vec3::new(0., 0., 2.), Vec3::new(2., 4., 2.)]);
    assert!(poly.is_polynomial());
    let c = quarter_circle().with_endpoints(Vec2::new(2., 0.), Vec2::new(0., 3.));
    assert_eq!(c.start(), Vec2::new(2., 0.));
    assert_eq!(c.end(), Vec2::new(0., 3.));
    assert_eq!(c.control_points[1], quarter_circle().control_points[1]);
  }

  #[test]
  fn affine_map_is_exact_for_rational() {
    let c = quarter_circle();
    let scale = Vec2::new(2., -0.5);
    let offset = Vec2::new(3., 7.);
    let m = c.affine_map(scale, offset);
    for i in 0..=20 {
      let t = i as f64 / 20.;
      let p = c.eval(t);
      let expected = Vec2::new(p.x * scale.x + offset.x, p.y * scale.y + offset.y);
      assert!(dist2(m.eval(t), expected) < 1e-14);
    }
    let tr = c.translate(Vec2::new(1., 1.));
    assert!(dist2(tr.eval(0.5), c.eval(0.5) + Vec2::new(1., 1.)) < 1e-15);
  }

  #[test]
  fn bbox_contains_curve() {
    let c = quarter_circle().affine_map(Vec2::new(1., 2.), Vec2::new(-1., 0.));
    let (min, max) = c.control_bbox();
    assert_eq!(min, Vec2::new(-1., 0.));
    assert_eq!(max, Vec2::new(0., 2.));
    for i in 0..=20 {
      let p = c.eval(i as f64 / 20.);
      assert!(p.x >= min.x - 1e-15 && p.x <= max.x + 1e-15);
      assert!(p.y >= min.y - 1e-15 && p.y <= max.y + 1e-15);
    }
  }

  #[test]
  fn transform_applies_matrix() {
    let c = rational_cubic3();
    let mat = Mat4::translate((1., 2., 3.)) * Mat4::rotate_z(0.7) * Mat4::scale((2., 2., 2.));
    let tc = c.transform(mat);
    for i in 0..=20 {
      let t = i as f64 / 20.;
      let p = c.eval(t);
      let expected = (mat * Vec4::new(p.x, p.y, p.z, 1.)).to_euclid();
      assert!(dist3(tc.eval(t), expected) < 1e-12);
    }
    let moved = c.transform(Mat4::translate((1., 0., 0.)));
    assert!(dist3(moved.eval(0.4), c.eval(0.4) + Vec3::new(1., 0., 0.)) < 1e-13);
  }
}
