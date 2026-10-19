use rendiation_algebra::*;

use super::*;
use crate::{ConvertError, ConvertResult};

/// Expand STEP style (distinct knots, multiplicities) into a full knot vector.
pub fn expand_knots(knots: &[f64], multiplicities: &[i64]) -> ConvertResult<Vec<f64>> {
  if knots.len() != multiplicities.len() {
    return Err(ConvertError::invalid(
      None,
      format!(
        "knot count {} does not match multiplicity count {}",
        knots.len(),
        multiplicities.len()
      ),
    ));
  }
  let mut out = Vec::new();
  for (i, (k, m)) in knots.iter().zip(multiplicities).enumerate() {
    if !k.is_finite() {
      return Err(ConvertError::invalid(
        None,
        format!("knot {k} is not finite"),
      ));
    }
    if i > 0 && knots[i - 1] > *k {
      return Err(ConvertError::invalid(None, "knots are not non decreasing"));
    }
    let m = usize::try_from(*m)
      .map_err(|_| ConvertError::invalid(None, format!("negative knot multiplicity {m}")))?;
    if out.len() + m > MAX_KNOTS {
      return Err(ConvertError::invalid(
        None,
        "knot vector is unreasonably long",
      ));
    }
    out.extend(std::iter::repeat_n(*k, m));
  }
  Ok(out)
}

/// Guards allocations against corrupt multiplicities.
const MAX_KNOTS: usize = 1 << 24;

/// Non-uniform rational B-spline curve. `knots.len() == control_points.len() + degree + 1`.
/// Clamped, unclamped and periodic (as unclamped knot vectors) are all valid input.
#[derive(Debug, Clone)]
pub struct NurbsCurve<P: HomogeneousPoint> {
  pub degree: usize,
  pub knots: Vec<f64>,
  pub control_points: Vec<P>,
}

pub type NurbsCurve2 = NurbsCurve<Vec3<f64>>;
pub type NurbsCurve3 = NurbsCurve<Vec4<f64>>;

impl<P: HomogeneousPoint> NurbsCurve<P> {
  /// Validates the knot vector (length, monotonicity, non empty domain) and the weights
  /// (finite and positive).
  pub fn new(degree: usize, knots: Vec<f64>, control_points: Vec<P>) -> ConvertResult<Self> {
    validate_knot_vector(degree, &knots, control_points.len())?;
    validate_weights(control_points.iter().map(|p| p.weight()))?;
    Ok(Self {
      degree,
      knots,
      control_points,
    })
  }

  /// Valid parameter domain `[knots[degree], knots[n]]`.
  pub fn domain(&self) -> (f64, f64) {
    (
      self.knots[self.degree],
      self.knots[self.control_points.len()],
    )
  }

  /// Parameters outside the domain extrapolate with the first / last span polynomial.
  pub fn eval(&self, t: f64) -> P::Euclid {
    let k = find_span(self.degree, &self.knots, self.control_points.len(), t);
    de_boor(self.degree, &self.knots, self.local(k), k, t).to_euclid()
  }

  /// Point and first derivative (Euclidean).
  pub fn eval_with_derivative(&self, t: f64) -> (P::Euclid, P::Euclid) {
    let k = find_span(self.degree, &self.knots, self.control_points.len(), t);
    let (h, dh) = de_boor_with_derivative(self.degree, &self.knots, self.local(k), k, t);
    rational_first_derivative(h, dh)
  }

  /// Exact decomposition into Bézier segments, one per non-empty knot span of the domain, each
  /// paired with its knot interval. Works for unclamped knot vectors.
  pub fn to_bezier_segments(&self) -> Vec<((f64, f64), RationalBezierCurve<P>)> {
    let spans = non_empty_spans(self.degree, &self.knots, self.control_points.len());
    decompose(self.degree, &self.knots, &self.control_points, &spans)
      .into_iter()
      .zip(&spans)
      .map(|(cp, k)| {
        let range = (self.knots[*k], self.knots[*k + 1]);
        (range, RationalBezierCurve::new(cp))
      })
      .collect()
  }

  fn local(&self, k: usize) -> &[P] {
    &self.control_points[k - self.degree..=k]
  }
}

/// Tensor product NURBS surface. Control point layout `control_points[v * count_u + u]`.
#[derive(Debug, Clone)]
pub struct NurbsSurface {
  pub degree_u: usize,
  pub degree_v: usize,
  pub knots_u: Vec<f64>,
  pub knots_v: Vec<f64>,
  pub count_u: usize,
  pub count_v: usize,
  pub control_points: Vec<Vec4<f64>>,
}

impl NurbsSurface {
  pub fn new(
    degree_u: usize,
    degree_v: usize,
    knots_u: Vec<f64>,
    knots_v: Vec<f64>,
    count_u: usize,
    count_v: usize,
    control_points: Vec<Vec4<f64>>,
  ) -> ConvertResult<Self> {
    if count_u.checked_mul(count_v) != Some(control_points.len()) {
      return Err(ConvertError::invalid(
        None,
        format!(
          "{} control points do not form a {count_u} x {count_v} grid",
          control_points.len()
        ),
      ));
    }
    validate_knot_vector(degree_u, &knots_u, count_u)?;
    validate_knot_vector(degree_v, &knots_v, count_v)?;
    validate_weights(control_points.iter().map(|p| p.w))?;
    Ok(Self {
      degree_u,
      degree_v,
      knots_u,
      knots_v,
      count_u,
      count_v,
      control_points,
    })
  }

  pub fn domain(&self) -> [(f64, f64); 2] {
    [
      (self.knots_u[self.degree_u], self.knots_u[self.count_u]),
      (self.knots_v[self.degree_v], self.knots_v[self.count_v]),
    ]
  }

  /// Distinct span boundaries of the domain per axis (first and last are the domain ends).
  pub fn span_breaks(&self, axis: Axis) -> Vec<f64> {
    let (degree, knots, count) = self.axis(axis);
    let spans = non_empty_spans(degree, knots, count);
    let mut breaks = Vec::with_capacity(spans.len() + 1);
    if let Some(first) = spans.first() {
      breaks.push(knots[*first]);
    }
    breaks.extend(spans.iter().map(|k| knots[k + 1]));
    breaks
  }

  pub fn eval(&self, u: f64, v: f64) -> Vec3<f64> {
    let (ku, kv) = self.find_spans(u, v);
    let column: Vec<_> = (kv - self.degree_v..=kv)
      .map(|j| de_boor(self.degree_u, &self.knots_u, self.local_row(j, ku), ku, u))
      .collect();
    de_boor(self.degree_v, &self.knots_v, &column, kv, v).to_euclid()
  }

  pub fn eval_with_partials(&self, u: f64, v: f64) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (ku, kv) = self.find_spans(u, v);
    let (column, column_du): (Vec<_>, Vec<_>) = (kv - self.degree_v..=kv)
      .map(|j| de_boor_with_derivative(self.degree_u, &self.knots_u, self.local_row(j, ku), ku, u))
      .unzip();
    let (h, h_v) = de_boor_with_derivative(self.degree_v, &self.knots_v, &column, kv, v);
    let h_u = de_boor(self.degree_v, &self.knots_v, &column_du, kv, v);
    let (p, du) = rational_first_derivative(h, h_u);
    let (_, dv) = rational_first_derivative(h, h_v);
    (p, du, dv)
  }

  /// Exact decomposition into Bézier patches indexed `[v_span][u_span]`, spans as in
  /// `span_breaks`. Works for unclamped knot vectors.
  pub fn to_bezier_patches(&self) -> Vec<Vec<RationalBezierSurface>> {
    let (pu, pv) = (self.degree_u, self.degree_v);
    let spans_u = non_empty_spans(pu, &self.knots_u, self.count_u);
    let spans_v = non_empty_spans(pv, &self.knots_v, self.count_v);

    // rows[j][su][i]: row j decomposed in u.
    let rows: Vec<_> = self
      .control_points
      .chunks_exact(self.count_u)
      .map(|row| decompose(pu, &self.knots_u, row, &spans_u))
      .collect();

    let mut patches: Vec<Vec<_>> = spans_v
      .iter()
      .map(|_| {
        spans_u
          .iter()
          .map(|_| vec![Vec4::new(0., 0., 0., 0.); (pu + 1) * (pv + 1)])
          .collect()
      })
      .collect();
    for su in 0..spans_u.len() {
      for i in 0..=pu {
        let column: Vec<_> = rows.iter().map(|row| row[su][i]).collect();
        for (sv, segment) in decompose(pv, &self.knots_v, &column, &spans_v)
          .into_iter()
          .enumerate()
        {
          for (j, p) in segment.into_iter().enumerate() {
            patches[sv][su][j * (pu + 1) + i] = p;
          }
        }
      }
    }
    patches
      .into_iter()
      .map(|row| {
        row
          .into_iter()
          .map(|cp| RationalBezierSurface::new(pu, pv, cp))
          .collect()
      })
      .collect()
  }

  fn axis(&self, axis: Axis) -> (usize, &[f64], usize) {
    match axis {
      Axis::U => (self.degree_u, &self.knots_u, self.count_u),
      Axis::V => (self.degree_v, &self.knots_v, self.count_v),
    }
  }

  fn find_spans(&self, u: f64, v: f64) -> (usize, usize) {
    (
      find_span(self.degree_u, &self.knots_u, self.count_u, u),
      find_span(self.degree_v, &self.knots_v, self.count_v, v),
    )
  }

  fn local_row(&self, j: usize, ku: usize) -> &[Vec4<f64>] {
    let row = j * self.count_u;
    &self.control_points[row + ku - self.degree_u..=row + ku]
  }
}

fn validate_knot_vector(degree: usize, knots: &[f64], count: usize) -> ConvertResult<()> {
  if degree == 0 {
    return Err(ConvertError::unsupported(None, "B-spline of degree 0"));
  }
  if count < degree + 1 {
    return Err(ConvertError::invalid(
      None,
      format!("{count} control points are too few for degree {degree}"),
    ));
  }
  if knots.len() != count + degree + 1 {
    return Err(ConvertError::invalid(
      None,
      format!(
        "knot vector length {} != control point count {count} + degree {degree} + 1",
        knots.len()
      ),
    ));
  }
  if knots.iter().any(|k| !k.is_finite()) {
    return Err(ConvertError::invalid(None, "knot vector is not finite"));
  }
  if knots.windows(2).any(|w| w[0] > w[1]) {
    return Err(ConvertError::invalid(
      None,
      "knot vector is not non decreasing",
    ));
  }
  if knots[degree] >= knots[count] {
    return Err(ConvertError::invalid(
      None,
      "B-spline parameter domain is empty",
    ));
  }
  Ok(())
}

fn validate_weights(weights: impl Iterator<Item = f64>) -> ConvertResult<()> {
  for w in weights {
    if !(w > 0. && w.is_finite()) {
      return Err(ConvertError::invalid(
        None,
        format!("invalid B-spline weight {w}"),
      ));
    }
  }
  Ok(())
}

/// Indices `k ∈ [degree, count)` with `knots[k] < knots[k + 1]`, ascending.
fn non_empty_spans(degree: usize, knots: &[f64], count: usize) -> Vec<usize> {
  (degree..count)
    .filter(|k| knots[*k] < knots[k + 1])
    .collect()
}

/// The non empty domain span containing `t` (`[knots[k], knots[k+1])`, the last span is closed).
/// Values outside the domain map to the first / last span.
fn find_span(degree: usize, knots: &[f64], count: usize, t: f64) -> usize {
  let (lo, hi) = (knots[degree], knots[count]);
  let k = if t >= hi {
    knots.partition_point(|x| *x < hi)
  } else {
    let t = t.max(lo);
    knots.partition_point(|x| *x <= t)
  };
  k.saturating_sub(1).clamp(degree, count - 1)
}

/// de Boor triangle on the `degree + 1` control points `d` of span `k`, levels `1..=levels`,
/// level `r` using parameter `arg(r)`. With distinct level arguments this evaluates the blossom.
fn de_boor_levels<T: Blend>(
  degree: usize,
  knots: &[f64],
  d: &mut [T],
  k: usize,
  levels: usize,
  arg: impl Fn(usize) -> f64,
) {
  for r in 1..=levels {
    let t = arg(r);
    for j in (r..=degree).rev() {
      let i = k + j - degree;
      let (a, b) = (knots[i], knots[i + degree + 1 - r]);
      d[j] = blend(d[j - 1], d[j], (t - a) / (b - a));
    }
  }
}

fn de_boor<T: Blend>(degree: usize, knots: &[f64], local: &[T], k: usize, t: f64) -> T {
  let mut d = local.to_vec();
  de_boor_levels(degree, knots, &mut d, k, degree, |_| t);
  d[degree]
}

fn de_boor_with_derivative<T: Blend>(
  degree: usize,
  knots: &[f64],
  local: &[T],
  k: usize,
  t: f64,
) -> (T, T) {
  let mut d = local.to_vec();
  de_boor_levels(degree, knots, &mut d, k, degree - 1, |_| t);
  let (a, b) = (d[degree - 1], d[degree]);
  let (u0, u1) = (knots[k], knots[k + 1]);
  (
    blend(a, b, (t - u0) / (u1 - u0)),
    (b - a) * (degree as f64 / (u1 - u0)),
  )
}

/// Bézier control points of each span in `spans`.
///
/// The Bézier point `j` of span `[a, b]` is the blossom `f(a^(degree-j), b^j)`, evaluated with a
/// de Boor triangle on the span's own `degree + 1` control points. This is exactly what Boehm
/// insertion of `a` and `b` up to multiplicity `degree` produces (including the clamping of an
/// unclamped knot vector at the domain ends), without touching control points outside the
/// span. Where the curve is continuous (shared knot multiplicity `<= degree`) the first point
/// of a span is copied from the last point of the previous one, so segments join bitwise.
fn decompose<T: Blend>(degree: usize, knots: &[f64], cps: &[T], spans: &[usize]) -> Vec<Vec<T>> {
  let mut out: Vec<Vec<T>> = Vec::with_capacity(spans.len());
  for (s, &k) in spans.iter().enumerate() {
    let (a, b) = (knots[k], knots[k + 1]);
    let local = &cps[k - degree..=k];
    let mut segment: Vec<_> = (0..=degree)
      .map(|j| {
        let mut d = local.to_vec();
        de_boor_levels(degree, knots, &mut d, k, degree, |r| {
          if r <= degree - j { a } else { b }
        });
        d[degree]
      })
      .collect();
    if s > 0 {
      let multiplicity = knots.iter().filter(|x| **x == a).count();
      if multiplicity <= degree {
        segment[0] = out[s - 1][degree];
      }
    }
    out.push(segment);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Independent Cox-de Boor basis evaluation (half open spans), for cross checking.
  fn basis(i: usize, p: usize, knots: &[f64], t: f64) -> f64 {
    if p == 0 {
      return if knots[i] <= t && t < knots[i + 1] {
        1.
      } else {
        0.
      };
    }
    let mut v = 0.;
    let d0 = knots[i + p] - knots[i];
    if d0 > 0. {
      v += (t - knots[i]) / d0 * basis(i, p - 1, knots, t);
    }
    let d1 = knots[i + p + 1] - knots[i + 1];
    if d1 > 0. {
      v += (knots[i + p + 1] - t) / d1 * basis(i + 1, p - 1, knots, t);
    }
    v
  }

  fn reference_eval<P: HomogeneousPoint>(c: &NurbsCurve<P>, t: f64) -> P::Euclid {
    let mut acc = c.control_points[0] * 0.;
    for (i, p) in c.control_points.iter().enumerate() {
      acc = acc + *p * basis(i, c.degree, &c.knots, t);
    }
    acc.to_euclid()
  }

  fn d3(a: Vec3<f64>, b: Vec3<f64>) -> f64 {
    (a - b).length()
  }

  fn sample_points(n: usize) -> Vec<Vec4<f64>> {
    (0..n)
      .map(|i| {
        let x = i as f64;
        let w = 1. + 0.5 * (x * 1.3).sin().abs();
        Vec4::from_euclid(Vec3::new(x, (x * 0.7).sin() * 3., (x * 0.3).cos()), w)
      })
      .collect()
  }

  /// Checks that the segments cover the domain, join, and match both evaluators.
  fn check_decomposition(c: &NurbsCurve3) {
    let segments = c.to_bezier_segments();
    let (lo, hi) = c.domain();
    assert_eq!(segments.first().unwrap().0.0, lo);
    assert_eq!(segments.last().unwrap().0.1, hi);
    for w in segments.windows(2) {
      assert_eq!(w[0].0.1, w[1].0.0);
    }
    for ((a, b), seg) in &segments {
      assert!(a < b);
      assert_eq!(seg.degree(), c.degree);
      for i in 0..=16 {
        let s = i as f64 / 16.;
        let t = a + (b - a) * s;
        let p = seg.eval(s);
        // At a discontinuous knot `eval` takes the right span, compare only left of it.
        if t < *b || *b == hi {
          assert!(d3(p, c.eval(t)) < 1e-12, "{t}");
        }
        if t < *b {
          assert!(d3(p, reference_eval(c, t)) < 1e-12, "{t}");
        }
      }
    }
  }

  #[test]
  fn expand() {
    assert_eq!(
      expand_knots(&[0., 1., 2.], &[3, 1, 3]).unwrap(),
      vec![0., 0., 0., 1., 2., 2., 2.]
    );
    assert_eq!(expand_knots(&[0., 1.], &[2, 0]).unwrap(), vec![0., 0.]);
    assert!(expand_knots(&[0., 1.], &[2]).is_err());
    assert!(expand_knots(&[0., 1.], &[2, -1]).is_err());
    assert!(expand_knots(&[1., 0.], &[2, 2]).is_err());
    assert!(expand_knots(&[0., f64::NAN], &[2, 2]).is_err());
    assert!(expand_knots(&[0., 1.], &[i64::MAX, 2]).is_err());
  }

  #[test]
  fn validation() {
    let cp = sample_points(4);
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 0., 1., 1., 1., 1.], cp.clone()).is_ok());
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 0., 1., 1., 1.], cp.clone()).is_err());
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 1., 0., 1., 1., 1.], cp.clone()).is_err());
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 0., 0., 1., 1., 1.], cp.clone()).is_err());
    assert!(NurbsCurve3::new(0, vec![0., 1., 2., 3., 4.], cp.clone()).is_err());
    assert!(NurbsCurve3::new(4, vec![0.; 9], cp.clone()).is_err());
    let mut bad = cp.clone();
    bad[1].w = 0.;
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 0., 1., 1., 1., 1.], bad).is_err());
    let mut bad = cp;
    bad[2].w = f64::NAN;
    assert!(NurbsCurve3::new(3, vec![0., 0., 0., 0., 1., 1., 1., 1.], bad).is_err());
    assert!(
      NurbsSurface::new(
        1,
        1,
        vec![0., 0., 1., 1.],
        vec![0., 0., 1., 1.],
        2,
        2,
        sample_points(3)
      )
      .is_err()
    );
  }

  #[test]
  fn clamped_with_interior_multiplicities() {
    let knots = vec![0., 0., 0., 0., 0.5, 1., 1., 2., 3., 3., 3., 3.];
    let c = NurbsCurve3::new(3, knots, sample_points(8)).unwrap();
    assert_eq!(c.domain(), (0., 3.));
    let segments = c.to_bezier_segments();
    assert_eq!(segments.len(), 4);
    check_decomposition(&c);
    // Clamped: the chain starts and ends on the first / last control point.
    assert_eq!(segments[0].1.control_points[0], c.control_points[0]);
    assert_eq!(
      *segments[3].1.control_points.last().unwrap(),
      c.control_points[7]
    );
    for w in segments.windows(2) {
      assert_eq!(w[0].1.control_points[3], w[1].1.control_points[0]);
    }
  }

  #[test]
  fn unclamped_uniform() {
    for degree in 1..=5 {
      let n = degree + 4;
      let knots = (0..n + degree + 1).map(|i| i as f64 * 0.25 - 1.).collect();
      let c = NurbsCurve3::new(degree, knots, sample_points(n)).unwrap();
      assert_eq!(c.to_bezier_segments().len(), n - degree);
      check_decomposition(&c);
    }
  }

  #[test]
  fn unclamped_non_uniform_with_zero_spans() {
    // Repeated knots inside the domain and at the domain ends of an unclamped vector.
    let knots = vec![-1., -0.5, 0., 0., 0.3, 0.3, 0.3, 1., 1.5, 1.5, 2.5];
    let c = NurbsCurve3::new(3, knots, sample_points(7)).unwrap();
    assert_eq!(c.domain(), (0., 1.));
    let segments = c.to_bezier_segments();
    let ranges: Vec<_> = segments.iter().map(|s| s.0).collect();
    assert_eq!(ranges, vec![(0., 0.3), (0.3, 1.)]);
    check_decomposition(&c);
  }

  #[test]
  fn periodic_closed_curve() {
    // Uniform periodic cubic: wrap the first `degree` control points.
    let degree = 3;
    let base: Vec<_> = (0..6)
      .map(|i| {
        let a = i as f64 * std::f64::consts::TAU / 6.;
        Vec4::from_euclid(
          Vec3::new(a.cos(), a.sin(), 0.2 * a.cos()),
          1. + 0.2 * i as f64,
        )
      })
      .collect();
    let mut cp = base.clone();
    cp.extend_from_slice(&base[..degree]);
    let knots = (0..cp.len() + degree + 1).map(|i| i as f64).collect();
    let c = NurbsCurve3::new(degree, knots, cp).unwrap();
    let (lo, hi) = c.domain();
    assert!(d3(c.eval(lo), c.eval(hi)) < 1e-13);
    check_decomposition(&c);
    let segments = c.to_bezier_segments();
    assert_eq!(segments.len(), 6);
    let first = segments[0].1.start();
    let last = segments[5].1.end();
    assert!(d3(first, last) < 1e-13);
  }

  #[test]
  fn discontinuous_interior_knot() {
    let knots = vec![0., 0., 0., 1., 1., 1., 2., 2., 2.];
    let c = NurbsCurve3::new(2, knots, sample_points(6)).unwrap();
    check_decomposition(&c);
    let segments = c.to_bezier_segments();
    assert_eq!(segments[0].1.control_points[2], c.control_points[2]);
    assert_eq!(segments[1].1.control_points[0], c.control_points[3]);
  }

  #[test]
  fn curve_2d_and_derivative() {
    let cp: Vec<_> = (0..5)
      .map(|i| {
        Vec3::from_euclid(
          Vec2::new(i as f64, (i * i) as f64 * 0.1),
          1. + i as f64 * 0.3,
        )
      })
      .collect();
    let knots = vec![0., 0.1, 0.2, 0.3, 0.6, 0.8, 1.0, 1.2];
    let c = NurbsCurve2::new(2, knots, cp).unwrap();
    for ((a, b), seg) in c.to_bezier_segments() {
      for i in 0..=8 {
        let s = i as f64 / 8.;
        let t = a + (b - a) * s;
        assert!((seg.eval(s) - c.eval(t)).length() < 1e-13);
        let (_, d) = c.eval_with_derivative(t);
        let (_, ds) = seg.eval_with_derivative(s);
        assert!((d * (b - a) - ds).length() < 1e-11);
      }
    }
    let h = 1e-6;
    let (_, d) = c.eval_with_derivative(0.5);
    let fd = (c.eval(0.5 + h) - c.eval(0.5 - h)) * (0.5 / h);
    assert!((d - fd).length() < 1e-6);
  }

  fn test_surface() -> NurbsSurface {
    // Unclamped cubic in u, clamped quadratic with a double knot in v.
    let count_u = 6;
    let count_v = 5;
    let knots_u = (0..count_u + 4).map(|i| i as f64 * 0.5).collect();
    let knots_v = vec![0., 0., 0., 1., 1., 3., 3., 3.];
    let mut cp = Vec::new();
    for j in 0..count_v {
      for i in 0..count_u {
        let (x, y) = (i as f64, j as f64);
        let w = 1. + 0.3 * ((x + 2. * y) * 0.9).sin().abs();
        cp.push(Vec4::from_euclid(
          Vec3::new(x, y, (x * 0.5).sin() * (y * 0.7).cos()),
          w,
        ));
      }
    }
    NurbsSurface::new(3, 2, knots_u, knots_v, count_u, count_v, cp).unwrap()
  }

  #[test]
  fn surface_decomposition() {
    let s = test_surface();
    let [du, dv] = s.domain();
    assert_eq!(du, (1.5, 3.));
    assert_eq!(dv, (0., 3.));
    let bu = s.span_breaks(Axis::U);
    let bv = s.span_breaks(Axis::V);
    assert_eq!(bu, vec![1.5, 2., 2.5, 3.]);
    assert_eq!(bv, vec![0., 1., 3.]);
    let patches = s.to_bezier_patches();
    assert_eq!(patches.len(), bv.len() - 1);
    for (sv, row) in patches.iter().enumerate() {
      assert_eq!(row.len(), bu.len() - 1);
      for (su, patch) in row.iter().enumerate() {
        let (u0, u1) = (bu[su], bu[su + 1]);
        let (v0, v1) = (bv[sv], bv[sv + 1]);
        for i in 0..=6 {
          for j in 0..=6 {
            let (a, b) = (i as f64 / 6., j as f64 / 6.);
            let (u, v) = (u0 + (u1 - u0) * a, v0 + (v1 - v0) * b);
            assert!(d3(patch.eval(a, b), s.eval(u, v)) < 1e-12);
            let (p, pu, pv) = s.eval_with_partials(u, v);
            let (q, qu, qv) = patch.eval_with_partials(a, b);
            assert!(d3(p, q) < 1e-12);
            assert!(d3(pu * (u1 - u0), qu) < 1e-10);
            // v = 1 is a C0 knot where `eval_with_partials` takes the upper span.
            if b < 1. || v1 == dv.1 {
              assert!(d3(pv * (v1 - v0), qv) < 1e-10);
            }
          }
        }
      }
    }
    // Shared patch boundaries are bitwise identical.
    for row in &patches {
      for w in row.windows(2) {
        for j in 0..=2 {
          assert_eq!(w[0].control_point(3, j), w[1].control_point(0, j));
        }
      }
    }
    for w in patches.windows(2) {
      for i in 0..=3 {
        assert_eq!(w[0][0].control_point(i, 2), w[1][0].control_point(i, 0));
      }
    }
  }

  #[test]
  fn surface_reference_eval_and_partials() {
    let s = test_surface();
    let h = 1e-6;
    for i in 1..10 {
      for j in 1..10 {
        let u = 1.5 + 1.5 * i as f64 / 10.;
        let v = 3. * j as f64 / 10.;
        let mut acc = Vec4::new(0., 0., 0., 0.);
        for jj in 0..s.count_v {
          for ii in 0..s.count_u {
            let b = basis(ii, 3, &s.knots_u, u) * basis(jj, 2, &s.knots_v, v);
            acc += s.control_points[jj * s.count_u + ii] * b;
          }
        }
        let (p, du, dv) = s.eval_with_partials(u, v);
        assert!(d3(p, acc.to_euclid()) < 1e-12);
        assert!(d3(p, s.eval(u, v)) < 1e-14);
        let fu = (s.eval(u + h, v) - s.eval(u - h, v)) * (0.5 / h);
        let fv = (s.eval(u, v + h) - s.eval(u, v - h)) * (0.5 / h);
        assert!(d3(du, fu) < 1e-6 * (1. + du.length()));
        // v = 1 is a C0 knot, skip the finite difference check across it.
        if (v - 1.).abs() > 2. * h {
          assert!(d3(dv, fv) < 1e-6 * (1. + dv.length()));
        }
      }
    }
  }

  #[test]
  fn random_knot_vectors() {
    let mut state = 12345u64;
    let mut rng = move || {
      state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
      (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut checked = 0;
    for case in 0..200 {
      let degree = 1 + case % 6;
      let count = degree + 1 + (rng() * 8.) as usize;
      // Random non decreasing knots with interior multiplicities up to `degree + 1`.
      let mut knots = Vec::new();
      let mut k = rng() * 2. - 1.;
      while knots.len() < count + degree + 1 {
        let m = 1 + (rng() * (degree + 1) as f64) as usize;
        for _ in 0..m.min(count + degree + 1 - knots.len()) {
          knots.push(k);
        }
        k += 0.1 + rng();
      }
      let cp: Vec<_> = (0..count)
        .map(|_| {
          let p = Vec3::new(rng() * 10., rng() * 10., rng() * 10.);
          Vec4::from_euclid(p, 0.3 + rng() * 2.)
        })
        .collect();
      let Ok(c) = NurbsCurve3::new(degree, knots, cp) else {
        continue;
      };
      check_decomposition(&c);
      checked += 1;
    }
    assert!(checked > 150);
  }
}
