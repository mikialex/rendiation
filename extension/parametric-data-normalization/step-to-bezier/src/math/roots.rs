use super::*;

/// All parameters `t ∈ [0, 1]` where the curve's `axis` coordinate equals `value`, ascending,
/// deduplicated within `t_eps`. Includes tangential touches and endpoints that lie on the line.
/// Robust Bernstein subdivision (convex hull) plus Newton polishing; never misses a crossing.
///
/// The roots are those of the homogeneous numerator `x_h(t) - value·w(t)` in Bernstein form.
/// Root isolation uses the Bernstein variation diminishing property (no sign change in the
/// coefficients means no root in the open interval, exactly one change means exactly one simple
/// root); intervals that keep several sign changes are bisected until they are shorter than
/// `t_eps / 4`, where a remaining (near) multiple root is reported when the numerator is zero
/// within rounding.
///
/// Zero means `|numerator| <= 1e-12 · scale`, with `scale` the largest coefficient magnitude
/// before cancellation. Degenerate case: if the whole curve lies on the line (all coefficients
/// zero), the result is just the endpoints `[0, 1]`; callers must detect such on-line curves by
/// other means (for example by testing the curve midpoint) if they need to.
pub fn axis_line_crossings(
  curve: &RationalBezierCurve2,
  axis: Axis,
  value: f64,
  t_eps: f64,
) -> Vec<f64> {
  let coeffs: Vec<_> = curve
    .control_points
    .iter()
    .map(|p| axis_coord(*p, axis) - value * p.z)
    .collect();
  let scale = curve
    .control_points
    .iter()
    .map(|p| axis_coord(*p, axis).abs() + (value * p.z).abs())
    .fold(0., f64::max);
  bernstein_roots(&coeffs, scale * ZERO_REL, t_eps)
}

const ZERO_REL: f64 = 1e-12;

fn axis_coord(p: rendiation_algebra::Vec3<f64>, axis: Axis) -> f64 {
  match axis {
    Axis::U => p.x,
    Axis::V => p.y,
  }
}

/// Roots in `[0, 1]` of the Bernstein polynomial with coefficients `coeffs`, see
/// `axis_line_crossings`. `zero_tol` is the absolute tolerance on polynomial values.
pub fn bernstein_roots(coeffs: &[f64], zero_tol: f64, t_eps: f64) -> Vec<f64> {
  if coeffs.is_empty() || coeffs.iter().any(|c| !c.is_finite()) {
    return Vec::new();
  }
  let n = coeffs.len();
  if coeffs.iter().all(|c| c.abs() <= zero_tol) {
    return vec![0., 1.];
  }
  let mut finder = RootFinder {
    global: coeffs,
    zero_tol,
    leaf_width: (t_eps * 0.25).max(1e-15),
    candidates: Vec::new(),
  };
  if coeffs[0].abs() <= zero_tol {
    finder.candidates.push(0.);
  }
  if coeffs[n - 1].abs() <= zero_tol {
    finder.candidates.push(1.);
  }
  if n > 1 {
    finder.isolate(coeffs.to_vec(), 0., 1., 0);
  }
  finder.finish(t_eps)
}

struct RootFinder<'a> {
  global: &'a [f64],
  zero_tol: f64,
  leaf_width: f64,
  candidates: Vec<f64>,
}

const MAX_DEPTH: usize = 64;

impl RootFinder<'_> {
  fn eval(&self, t: f64) -> f64 {
    de_casteljau(self.global, t)
  }

  /// `local` are the Bernstein coefficients restricted to `[lo, hi]`.
  fn isolate(&mut self, local: Vec<f64>, lo: f64, hi: f64, depth: usize) {
    let variations = sign_variations(&local);
    if variations == 0 {
      return;
    }
    if variations == 1 {
      let first_sign = local.iter().find(|c| **c != 0.).map_or(0., |c| c.signum());
      let root = self.polish_bracketed(lo, hi, first_sign);
      self.candidates.push(root);
      return;
    }
    if hi - lo <= self.leaf_width || depth >= MAX_DEPTH {
      self.leaf(lo, hi);
      return;
    }
    let mid = 0.5 * (lo + hi);
    let (left, right) = split_coefficients(&local);
    let mid_value = left[left.len() - 1];
    let (vl, vr) = (sign_variations(&left), sign_variations(&right));
    // A root exactly at (or within rounding of) the split point is invisible to both halves
    // when neither keeps a sign change around it.
    if vl == 0 && vr == 0 && mid_value.abs() <= self.zero_tol {
      self.candidates.push(mid);
    }
    if vl > 0 {
      self.isolate(left, lo, mid, depth + 1);
    }
    if vr > 0 {
      self.isolate(right, mid, hi, depth + 1);
    }
  }

  /// Tiny interval that still has several coefficient sign changes: a (near) multiple root or a
  /// cluster of roots closer than the leaf width.
  fn leaf(&mut self, lo: f64, hi: f64) {
    const SAMPLES: usize = 8;
    let samples: Vec<_> = (0..=SAMPLES)
      .map(|i| {
        let t = lo + (hi - lo) * (i as f64 / SAMPLES as f64);
        (t, self.eval(t))
      })
      .collect();
    for w in samples.windows(2) {
      if w[0].1 * w[1].1 < 0. {
        let root = self.polish_bracketed(w[0].0, w[1].0, w[0].1.signum());
        self.candidates.push(root);
        return;
      }
    }
    let (t, v) = samples
      .iter()
      .copied()
      .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
      .unwrap_or((lo, f64::INFINITY));
    if v.abs() <= self.zero_tol {
      self.candidates.push(t);
    }
  }

  /// The single root in `(lo, hi)`, where the polynomial has sign `lo_sign` right of `lo` and the
  /// opposite sign left of `hi`. Newton steps safeguarded by bisection.
  fn polish_bracketed(&self, mut lo: f64, mut hi: f64, lo_sign: f64) -> f64 {
    let mut t = 0.5 * (lo + hi);
    for _ in 0..200 {
      let (f, df) = de_casteljau_with_derivative(self.global, t);
      if f == 0. {
        return t;
      }
      if f.signum() == lo_sign {
        lo = t;
      } else {
        hi = t;
      }
      if hi - lo <= f64::EPSILON * hi.abs().max(lo.abs()).max(1e-300) {
        break;
      }
      let newton = t - f / df;
      let next = if df != 0. && newton > lo && newton < hi {
        newton
      } else {
        0.5 * (lo + hi)
      };
      if next == t {
        break;
      }
      t = next;
    }
    t.clamp(0., 1.)
  }

  /// Sort, then merge clusters: neighbors closer than `t_eps`, or between which the polynomial
  /// stays zero within tolerance (a double root is only located to about the square root of
  /// the rounding level, rounding noise may report it as several nearby roots). A cluster keeps
  /// an exact endpoint 0 / 1 if it has one, otherwise its member with the smallest residual.
  fn finish(mut self, t_eps: f64) -> Vec<f64> {
    self
      .candidates
      .iter_mut()
      .for_each(|t| *t = t.clamp(0., 1.));
    self.candidates.sort_by(f64::total_cmp);
    let mut out: Vec<f64> = Vec::new();
    let mut cluster: Vec<f64> = Vec::new();
    let flush = |cluster: &mut Vec<f64>, out: &mut Vec<f64>| {
      if cluster.is_empty() {
        return;
      }
      let best = if cluster.contains(&0.) {
        0.
      } else if cluster.contains(&1.) {
        1.
      } else {
        cluster
          .iter()
          .copied()
          .min_by(|a, b| self.eval(*a).abs().total_cmp(&self.eval(*b).abs()))
          .unwrap_or(cluster[0])
      };
      out.push(best);
      cluster.clear();
    };
    for t in self.candidates.iter().copied() {
      if let Some(last) = cluster.last()
        && t - last > t_eps
        && !self.zero_between(*last, t)
      {
        flush(&mut cluster, &mut out);
      }
      cluster.push(t);
    }
    flush(&mut cluster, &mut out);
    out
  }
}

impl RootFinder<'_> {
  fn zero_between(&self, a: f64, b: f64) -> bool {
    (1..8).all(|i| self.eval(a + (b - a) * (i as f64 / 8.)).abs() <= self.zero_tol)
  }
}

fn sign_variations(c: &[f64]) -> usize {
  let mut count = 0;
  let mut prev = 0.;
  for v in c {
    if *v != 0. {
      if prev != 0. && (prev > 0.) != (*v > 0.) {
        count += 1;
      }
      prev = *v;
    }
  }
  count
}

/// de Casteljau split of scalar Bernstein coefficients at 0.5.
fn split_coefficients(c: &[f64]) -> (Vec<f64>, Vec<f64>) {
  let n = c.len();
  let mut work = c.to_vec();
  let mut left = Vec::with_capacity(n);
  let mut right = Vec::with_capacity(n);
  left.push(work[0]);
  right.push(work[n - 1]);
  for k in 1..n {
    for i in 0..n - k {
      work[i] = 0.5 * (work[i] + work[i + 1]);
    }
    left.push(work[0]);
    right.push(work[n - 1 - k]);
  }
  right.reverse();
  (left, right)
}

#[cfg(test)]
mod tests {
  use rendiation_algebra::*;

  use super::*;

  const EPS: f64 = 1e-9;

  fn binom(n: usize, k: usize) -> f64 {
    (0..k).fold(1., |b, i| b * (n - i) as f64 / (i + 1) as f64)
  }

  /// Power basis coefficients (ascending) to Bernstein coefficients of the same degree.
  fn power_to_bernstein(power: &[f64]) -> Vec<f64> {
    let n = power.len() - 1;
    (0..=n)
      .map(|i| {
        (0..=i)
          .map(|j| binom(i, j) / binom(n, j) * power[j])
          .sum::<f64>()
      })
      .collect()
  }

  fn bernstein_product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let (m, n) = (a.len() - 1, b.len() - 1);
    let mut out = vec![0.; m + n + 1];
    for (i, x) in a.iter().enumerate() {
      for (j, y) in b.iter().enumerate() {
        out[i + j] += binom(m, i) * binom(n, j) / binom(m + n, i + j) * x * y;
      }
    }
    out
  }

  /// Polynomial `scale · Π (t - r)` as Bernstein coefficients, built in Bernstein form (well
  /// conditioned, unlike the power basis).
  fn from_roots(roots: &[f64], scale: f64) -> Vec<f64> {
    roots
      .iter()
      .fold(vec![scale], |acc, r| bernstein_product(&acc, &[-r, 1. - r]))
  }

  /// Rational curve with non trivial weights whose numerator `x_h - value·w` has the Bernstein
  /// coefficients `b`; y is arbitrary.
  fn curve_from_bernstein(b: &[f64], value: f64) -> RationalBezierCurve2 {
    let n = b.len();
    RationalBezierCurve2::new(
      b.iter()
        .enumerate()
        .map(|(i, c)| {
          let w = 1. + 0.5 * (i as f64 * 1.7).sin().abs();
          Vec3::new(c + value * w, i as f64 / n as f64 * w, w)
        })
        .collect(),
    )
  }

  fn assert_roots(found: &[f64], expected: &[f64], tol: f64) {
    assert_eq!(
      found.len(),
      expected.len(),
      "found {found:?} expected {expected:?}"
    );
    for (a, b) in found.iter().zip(expected) {
      assert!(
        (a - b).abs() <= tol,
        "found {found:?} expected {expected:?}"
      );
    }
  }

  #[test]
  fn line_crossing() {
    let c = RationalBezierCurve2::line(Vec2::new(0., 0.), Vec2::new(4., 2.));
    assert_roots(&axis_line_crossings(&c, Axis::U, 1., EPS), &[0.25], 1e-15);
    assert_roots(&axis_line_crossings(&c, Axis::V, 1., EPS), &[0.5], 1e-15);
    assert!(axis_line_crossings(&c, Axis::U, 5., EPS).is_empty());
    assert_roots(&axis_line_crossings(&c, Axis::U, 0., EPS), &[0.], 0.);
    assert_roots(&axis_line_crossings(&c, Axis::U, 4., EPS), &[1.], 0.);
  }

  #[test]
  fn circle_crossings() {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let c = RationalBezierCurve2::from_euclid(
      &[Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)],
      Some(&[1., h, 1.]),
    );
    let r = axis_line_crossings(&c, Axis::U, 0.5, EPS);
    assert_eq!(r.len(), 1);
    let p = c.eval(r[0]);
    assert!((p.x - 0.5).abs() < 1e-15);
    assert!((p.y - 0.75f64.sqrt()).abs() < 1e-14);
    // Tangent touch: x = 1 at t = 0 only.
    assert_roots(&axis_line_crossings(&c, Axis::U, 1., EPS), &[0.], 0.);
  }

  #[test]
  fn simple_and_multiple_roots() {
    let value = 0.3;
    let cases: Vec<(Vec<f64>, Vec<f64>)> = vec![
      (vec![0.2, 0.5, 0.9], vec![0.2, 0.5, 0.9]),
      (vec![0.4, 0.4], vec![0.4]),
      (vec![0.5, 0.5], vec![0.5]),
      (vec![0.25, 0.25, 0.7], vec![0.25, 0.7]),
      (vec![0.6, 0.6, 0.6], vec![0.6]),
      (vec![0.1, 0.3, 0.3, 0.8, -0.5, 1.4], vec![0.1, 0.3, 0.8]),
      (vec![0., 0.5, 1.], vec![0., 0.5, 1.]),
      (vec![0., 0., 0.5], vec![0., 0.5]),
      (vec![0.3333, 0.33335], vec![0.3333, 0.33335]),
    ];
    for (roots, expected) in cases {
      let b = from_roots(&roots, 2.);
      let c = curve_from_bernstein(&b, value);
      let found = axis_line_crossings(&c, Axis::U, value, EPS);
      // A root of multiplicity m is only located to about the m-th root of the rounding level.
      assert_roots(&found, &expected, 2e-5);
      for t in found {
        assert!((c.eval(t).x - value).abs() < 1e-11);
      }
    }
  }

  #[test]
  fn simple_roots_are_precise() {
    let roots = [0.013, 0.2, 0.45, 0.61, 0.99];
    let c = curve_from_bernstein(&from_roots(&roots, -3.), 1.5);
    let found = axis_line_crossings(&c, Axis::U, 1.5, EPS);
    assert_roots(&found, &roots, 1e-12);
  }

  #[test]
  fn high_degree() {
    let roots: Vec<_> = (0..15).map(|i| (i as f64 + 0.5) / 15.).collect();
    let b = from_roots(&roots, 1e6);
    let c = curve_from_bernstein(&b, 0.);
    let found = axis_line_crossings(&c, Axis::U, 0., EPS);
    assert_roots(&found, &roots, 1e-11);

    // Degree 15 with only two crossings.
    let mut b = vec![1.; 16];
    b[7] = -40.;
    let c = curve_from_bernstein(&b, 2.);
    let found = axis_line_crossings(&c, Axis::U, 2., EPS);
    assert_eq!(found.len(), 2);
    for t in found {
      assert!((c.eval(t).x - 2.).abs() < 1e-12);
    }
  }

  #[test]
  fn near_tangent() {
    // Two crossings 2e-6 apart.
    let b = from_roots(&[0.5 - 1e-6, 0.5 + 1e-6], 1.);
    let c = curve_from_bernstein(&b, 0.);
    let found = axis_line_crossings(&c, Axis::U, 0., EPS);
    assert_roots(&found, &[0.5 - 1e-6, 0.5 + 1e-6], 1e-10);

    // Misses the line by a clearly non zero gap: no root.
    let mut power = vec![0.25 + 1e-6, -1., 1.];
    let c = curve_from_bernstein(&power_to_bernstein(&power), 0.);
    assert!(axis_line_crossings(&c, Axis::U, 0., EPS).is_empty());
    // Exactly touching at an off grid parameter.
    let r = 0.3141592653589793;
    power = vec![r * r, -2. * r, 1.];
    let c = curve_from_bernstein(&power_to_bernstein(&power), 0.);
    let found = axis_line_crossings(&c, Axis::U, 0., EPS);
    assert_roots(&found, &[r], 1e-6);
  }

  #[test]
  fn curve_on_line() {
    let c = RationalBezierCurve2::from_euclid(
      &[Vec2::new(2., 0.), Vec2::new(2., 1.), Vec2::new(2., 3.)],
      Some(&[1., 0.7, 1.3]),
    );
    assert_eq!(axis_line_crossings(&c, Axis::U, 2., EPS), vec![0., 1.]);
    assert!(axis_line_crossings(&c, Axis::U, 2.5, EPS).is_empty());
    let r = axis_line_crossings(&c, Axis::V, 1., EPS);
    assert_eq!(r.len(), 1);
    assert!((c.eval(r[0]).y - 1.).abs() < 1e-14);
  }

  #[test]
  fn endpoint_on_line_and_touch_back() {
    // Starts on the line, leaves, comes back and touches it, then leaves again.
    let b = from_roots(&[0., 0.6, 0.6], 1.);
    let c = curve_from_bernstein(&b, 1.);
    let found = axis_line_crossings(&c, Axis::U, 1., EPS);
    assert_eq!(found[0], 0.);
    assert_roots(&found, &[0., 0.6], 1e-6);
    // Ends on the line.
    let b = from_roots(&[0.2, 1.], 1.);
    let c = curve_from_bernstein(&b, 1.);
    let found = axis_line_crossings(&c, Axis::U, 1., EPS);
    assert_roots(&found, &[0.2, 1.], 1e-12);
    assert_eq!(found[1], 1.);
  }

  #[test]
  fn dedupe() {
    // Roots closer than t_eps collapse to one.
    let b = from_roots(&[0.5, 0.5 + 1e-11], 1.);
    let found = bernstein_roots(&b, 1e-14, 1e-9);
    assert_eq!(found.len(), 1);
    assert!((found[0] - 0.5).abs() < 1e-9);
  }

  struct Lcg(u64);
  impl Lcg {
    fn next(&mut self) -> f64 {
      self.0 = self
        .0
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
      (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
  }

  #[test]
  fn random_curves_never_miss_crossings() {
    let mut rng = Lcg(7);
    for case in 0..400 {
      let degree = 1 + case % 9;
      let points: Vec<_> = (0..=degree)
        .map(|_| Vec2::new(rng.next() * 4. - 2., rng.next() * 4. - 2.))
        .collect();
      let weights: Vec<_> = (0..=degree).map(|_| 0.2 + rng.next() * 3.).collect();
      let c = RationalBezierCurve2::from_euclid(&points, Some(&weights));
      let axis = if case % 2 == 0 { Axis::U } else { Axis::V };
      let value = rng.next() * 2. - 1.;
      let found = axis_line_crossings(&c, axis, value, EPS);
      let coord = |t: f64| {
        let p = c.eval(t);
        if axis == Axis::U { p.x } else { p.y }
      };
      for t in &found {
        assert!((coord(*t) - value).abs() < 1e-9, "case {case}");
      }
      assert!(found.windows(2).all(|w| w[1] - w[0] > EPS));
      let samples = 4000;
      for i in 0..samples {
        let (t0, t1) = (i as f64 / samples as f64, (i + 1) as f64 / samples as f64);
        let (f0, f1) = (coord(t0) - value, coord(t1) - value);
        if f0 * f1 < 0. {
          assert!(
            found.iter().any(|t| *t >= t0 - EPS && *t <= t1 + EPS),
            "case {case}: missed crossing in [{t0}, {t1}], found {found:?}"
          );
        }
      }
    }
  }

  #[test]
  fn random_tangent_touches() {
    let mut rng = Lcg(11);
    for case in 0..200 {
      let r = 0.05 + 0.9 * rng.next();
      let mut roots = vec![r, r];
      for _ in 0..case % 4 {
        roots.push(rng.next() * 1.4 - 0.2);
      }
      let b = from_roots(&roots, 1. + rng.next());
      let value = rng.next();
      let c = curve_from_bernstein(&b, value);
      let found = axis_line_crossings(&c, Axis::U, value, EPS);
      assert!(
        found.iter().any(|t| (t - r).abs() < 1e-6),
        "case {case}: touch at {r} missed, found {found:?}, roots {roots:?}"
      );
      let expected =
        roots
          .iter()
          .filter(|x| (0. ..=1.).contains(*x))
          .fold(Vec::<f64>::new(), |mut acc, x| {
            if !acc.iter().any(|y| (x - y).abs() < 1e-3) {
              acc.push(*x);
            }
            acc
          });
      assert!(
        found.len() <= expected.len(),
        "case {case}: {found:?} {roots:?}"
      );
    }
  }
}
