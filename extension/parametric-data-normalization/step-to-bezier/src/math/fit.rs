use rendiation_algebra::*;

use super::*;
use crate::{ConvertError, ConvertResult};

/// Adaptive approximation of a 2D parametric function by polynomial Bézier curves.
///
/// `f(s)` for `s ∈ range` is the target (for example a projected 3D edge expressed in P space),
/// it is continuous but may be expensive. The fitted pieces interpolate `f` exactly at their
/// endpoints (so a chain is exactly connected and ends exactly at `f(range.0)` / `f(range.1)`),
/// and are split recursively until `error(piece, sub_range) <= tol`. `error` is supplied by the
/// caller so the bound can be measured in model space (3D), not in parameter space.
///
/// Returns the pieces in order with their `s` sub ranges.
///
/// Each piece is a least squares fit of the interior control points to samples of `f`, with
/// uniform or chord length parameters improved by parameter correction (closest point) passes. The
/// piece parameter is therefore not an affine image of `s`: `error` should measure a
/// geometric deviation (for example distances to the closest curve point), not a parametric one.
/// Pieces split at the `s` midpoint, both halves share the very same `f(s_mid)` value.
pub fn fit_bezier_2d(
  f: &dyn Fn(f64) -> Vec2<f64>,
  range: (f64, f64),
  degree: usize,
  error: &dyn Fn(&RationalBezierCurve2, (f64, f64)) -> f64,
  tol: f64,
  max_depth: usize,
) -> ConvertResult<Vec<((f64, f64), RationalBezierCurve2)>> {
  if degree == 0 {
    return Err(ConvertError::unsupported(
      None,
      "fitting degree must be at least 1",
    ));
  }
  if !(range.0.is_finite() && range.1.is_finite()) {
    return Err(ConvertError::numerical(None, "fitting range is not finite"));
  }
  let fitter = Fitter {
    f,
    degree,
    error,
    tol,
    max_depth,
  };
  let mut out = Vec::new();
  let p0 = fitter.sample(range.0)?;
  let p1 = fitter.sample(range.1)?;
  fitter.fit(range, p0, p1, 0, &mut out)?;
  Ok(out)
}

struct Fitter<'a> {
  f: &'a dyn Fn(f64) -> Vec2<f64>,
  degree: usize,
  error: &'a dyn Fn(&RationalBezierCurve2, (f64, f64)) -> f64,
  tol: f64,
  max_depth: usize,
}

impl Fitter<'_> {
  fn sample(&self, s: f64) -> ConvertResult<Vec2<f64>> {
    let p = (self.f)(s);
    if p.x.is_finite() && p.y.is_finite() {
      Ok(p)
    } else {
      Err(ConvertError::numerical(
        None,
        format!("fitting target is not finite at s = {s}"),
      ))
    }
  }

  fn fit(
    &self,
    range: (f64, f64),
    p0: Vec2<f64>,
    p1: Vec2<f64>,
    depth: usize,
    out: &mut Vec<((f64, f64), RationalBezierCurve2)>,
  ) -> ConvertResult<()> {
    let curve = self.fit_piece(range, p0, p1)?;
    let err = (self.error)(&curve, range);
    if err <= self.tol {
      out.push((range, curve));
      return Ok(());
    }
    let mid = 0.5 * (range.0 + range.1);
    if depth >= self.max_depth || !(mid > range.0 && mid < range.1) {
      return Err(ConvertError::numerical(
        None,
        format!(
          "curve fit error {err} above tolerance {} over s in [{}, {}] at depth {depth}",
          self.tol, range.0, range.1
        ),
      ));
    }
    let pm = self.sample(mid)?;
    self.fit((range.0, mid), p0, pm, depth + 1, out)?;
    self.fit((mid, range.1), pm, p1, depth + 1, out)
  }

  fn fit_piece(
    &self,
    range: (f64, f64),
    p0: Vec2<f64>,
    p1: Vec2<f64>,
  ) -> ConvertResult<RationalBezierCurve2> {
    let d = self.degree;
    let line = RationalBezierCurve2::line(p0, p1);
    if d == 1 {
      return Ok(line);
    }
    let m = (4 * d).max(16);
    let mut points = Vec::with_capacity(m + 1);
    points.push(p0);
    for k in 1..m {
      points.push(self.sample(range.0 + (range.1 - range.0) * (k as f64 / m as f64))?);
    }
    points.push(p1);

    // Two starting parameterizations (uniform in `s` and chord length), each improved by
    // parameter correction, keep the fit with the smaller sample residual.
    let uniform = (0..=m).map(|k| k as f64 / m as f64).collect();
    let best = [uniform, chord_length_params(&points)]
      .into_iter()
      .filter_map(|params| fit_with_correction(&points, params, d))
      .min_by(|a, b| a.1.total_cmp(&b.1));
    Ok(match best {
      Some((curve, _)) => curve,
      None => elevate_to(line, d),
    })
  }
}

/// Distance minimization iterations after the initial fit.
const MAX_ITERATIONS: usize = 32;
/// Lower bound of the tangential residual weight, keeps the normal equations regular.
const MIN_TANGENTIAL_WEIGHT: f64 = 1e-8;

/// Parametric least squares fit followed by squared distance minimization (SDM, Wang, Pottmann
/// and Liu 2006) iterations: each step projects the samples onto the current curve, then solves
/// a linear least squares problem on the second order approximation of the squared distance,
/// the normal residual with weight 1 and the tangential one with weight `d / (d + ρ)` (distance
/// `d`, curvature radius `ρ`). Plain parameter correction converges far too slowly. Returns the
/// curve with the smallest max sample distance seen and that distance.
fn fit_with_correction(
  points: &[Vec2<f64>],
  mut params: Vec<f64>,
  degree: usize,
) -> Option<(RationalBezierCurve2, f64)> {
  let mut curve = least_squares(points, &params, degree)?;
  project_params(&curve, points, &mut params);
  let mut best = (residual(&curve, points, &params), curve.clone());
  let mut stalled = 0;
  for _ in 0..MAX_ITERATIONS {
    let Some(next) = distance_step(points, &params, &curve, degree) else {
      break;
    };
    curve = next;
    project_params(&curve, points, &mut params);
    let r = residual(&curve, points, &params);
    if r < best.0 * (1. - 1e-3) {
      stalled = 0;
    } else {
      stalled += 1;
    }
    if r < best.0 {
      best = (r, curve.clone());
    }
    if stalled >= 2 || best.0 == 0. {
      break;
    }
  }
  Some((best.1, best.0))
}

fn distance_step(
  points: &[Vec2<f64>],
  params: &[f64],
  curve: &RationalBezierCurve2,
  degree: usize,
) -> Option<RationalBezierCurve2> {
  let m = points.len() - 1;
  let (p0, p1) = (points[0], points[m]);
  let n = 2 * (degree - 1);
  let mut matrix = vec![0.; n * n];
  let mut rhs = vec![0.; n];
  let mut row = vec![0.; n];
  for k in 1..m {
    let u = params[k];
    let b = bernstein_basis(degree, u);
    let target = points[k] - p0 * b[0] - p1 * b[degree];
    let (c, d, dd) = curve.eval_with_second_derivative(u);
    let len = d.length();
    let directions = if len > 0. && len.is_finite() {
      let t = d * (1. / len);
      let distance = (c - points[k]).length();
      let curvature = (d.x * dd.y - d.y * dd.x).abs() / (len * len * len);
      let weight = (distance * curvature / (distance * curvature + 1.)).max(MIN_TANGENTIAL_WEIGHT);
      [Vec2::new(-t.y, t.x), t * weight.sqrt()]
    } else {
      [Vec2::new(1., 0.), Vec2::new(0., 1.)]
    };
    for e in directions {
      for i in 0..degree - 1 {
        row[2 * i] = b[i + 1] * e.x;
        row[2 * i + 1] = b[i + 1] * e.y;
      }
      let r = target.dot(e);
      for i in 0..n {
        rhs[i] += row[i] * r;
        for j in 0..n {
          matrix[i * n + j] += row[i] * row[j];
        }
      }
    }
  }
  let x = solve(matrix, rhs)?;
  let interior = x.as_chunks::<2>().0.iter().map(|[a, b]| Vec2::new(*a, *b));
  build_curve(p0, p1, interior)
}

fn residual(curve: &RationalBezierCurve2, points: &[Vec2<f64>], params: &[f64]) -> f64 {
  points
    .iter()
    .zip(params)
    .map(|(q, u)| (curve.eval(*u) - *q).length())
    .fold(0., f64::max)
}

fn elevate_to(mut curve: RationalBezierCurve2, degree: usize) -> RationalBezierCurve2 {
  while curve.degree() < degree {
    curve = curve.elevate_degree();
  }
  curve
}

/// Normalized cumulative chord length, uniform when the polyline is degenerate.
fn chord_length_params(points: &[Vec2<f64>]) -> Vec<f64> {
  let m = points.len() - 1;
  let mut params = Vec::with_capacity(m + 1);
  let mut acc = 0.;
  params.push(0.);
  for w in points.windows(2) {
    acc += (w[1] - w[0]).length();
    params.push(acc);
  }
  if acc > 0. && acc.is_finite() {
    params.iter_mut().for_each(|u| *u /= acc);
    params[m] = 1.;
  } else {
    params
      .iter_mut()
      .enumerate()
      .for_each(|(k, u)| *u = k as f64 / m as f64);
  }
  params
}

/// A few Newton steps per interior sample towards its closest curve parameter.
fn project_params(curve: &RationalBezierCurve2, points: &[Vec2<f64>], params: &mut [f64]) {
  let m = params.len() - 1;
  for k in 1..m {
    for _ in 0..3 {
      let u = params[k];
      let (c, d, dd) = curve.eval_with_second_derivative(u);
      let diff = c - points[k];
      let denominator = d.dot(d) + diff.dot(dd);
      if denominator <= 0. || denominator.is_nan() {
        break;
      }
      let next = (u - diff.dot(d) / denominator).clamp(0., 1.);
      if !next.is_finite() || next == u {
        break;
      }
      params[k] = next;
    }
  }
}

/// Least squares interior control points with exact endpoints `points[0]` / `points[m]`.
/// `None` when the normal equations are singular or the result is not finite.
fn least_squares(
  points: &[Vec2<f64>],
  params: &[f64],
  degree: usize,
) -> Option<RationalBezierCurve2> {
  let m = points.len() - 1;
  let (p0, p1) = (points[0], points[m]);
  let unknowns = degree - 1;
  let mut matrix = vec![0.; unknowns * unknowns];
  let mut rhs = vec![Vec2::new(0., 0.); unknowns];
  for (q, u) in points.iter().zip(params) {
    let b = bernstein_basis(degree, *u);
    let r = *q - p0 * b[0] - p1 * b[degree];
    for i in 0..unknowns {
      rhs[i] += r * b[i + 1];
      for j in 0..unknowns {
        matrix[i * unknowns + j] += b[i + 1] * b[j + 1];
      }
    }
  }
  build_curve(p0, p1, solve(matrix, rhs)?)
}

/// Polynomial curve from exact endpoints and interior points, `None` if anything is not finite.
fn build_curve(
  p0: Vec2<f64>,
  p1: Vec2<f64>,
  interior: impl IntoIterator<Item = Vec2<f64>>,
) -> Option<RationalBezierCurve2> {
  let mut cp = vec![Vec3::new(p0.x, p0.y, 1.)];
  for p in interior {
    if !(p.x.is_finite() && p.y.is_finite()) {
      return None;
    }
    cp.push(Vec3::new(p.x, p.y, 1.));
  }
  cp.push(Vec3::new(p1.x, p1.y, 1.));
  Some(RationalBezierCurve2::new(cp))
}

/// All degree `n` Bernstein basis values at `u`.
fn bernstein_basis(n: usize, u: f64) -> Vec<f64> {
  let mut b = vec![0.; n + 1];
  b[0] = 1.;
  for k in 1..=n {
    for i in (1..=k).rev() {
      b[i] = b[i] * (1. - u) + b[i - 1] * u;
    }
    b[0] *= 1. - u;
  }
  b
}

/// Gaussian elimination with partial pivoting on a dense `n x n` system (scalar or vector right
/// hand sides).
fn solve<T: Blend>(mut a: Vec<f64>, mut b: Vec<T>) -> Option<Vec<T>> {
  let n = b.len();
  let scale = a.iter().fold(0., |m: f64, v| m.max(v.abs()));
  if scale <= 0. || scale.is_nan() {
    return None;
  }
  for col in 0..n {
    let pivot = (col..n).max_by(|i, j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))?;
    if a[pivot * n + col].abs() <= scale * 1e-14 {
      return None;
    }
    if pivot != col {
      for k in 0..n {
        a.swap(col * n + k, pivot * n + k);
      }
      b.swap(col, pivot);
    }
    for row in col + 1..n {
      let factor = a[row * n + col] / a[col * n + col];
      for k in col..n {
        a[row * n + k] -= factor * a[col * n + k];
      }
      b[row] = b[row] - b[col] * factor;
    }
  }
  let mut x = b.clone();
  for row in (0..n).rev() {
    let mut acc = b[row];
    for k in row + 1..n {
      acc = acc - x[k] * a[row * n + k];
    }
    x[row] = acc * (1. / a[row * n + row]);
  }
  Some(x)
}

#[cfg(test)]
mod tests {
  use std::f64::consts::PI;

  use super::*;

  /// Geometric error against a unit circle: max radial deviation over dense curve samples.
  fn circle_error(c: &RationalBezierCurve2, _: (f64, f64)) -> f64 {
    (0..=64)
      .map(|i| (c.eval(i as f64 / 64.).length() - 1.).abs())
      .fold(0., f64::max)
  }

  fn check_chain(
    pieces: &[((f64, f64), RationalBezierCurve2)],
    f: &dyn Fn(f64) -> Vec2<f64>,
    range: (f64, f64),
  ) {
    assert_eq!(pieces[0].0.0, range.0);
    assert_eq!(pieces.last().unwrap().0.1, range.1);
    assert_eq!(pieces[0].1.start(), f(range.0));
    assert_eq!(pieces.last().unwrap().1.end(), f(range.1));
    for w in pieces.windows(2) {
      assert_eq!(w[0].0.1, w[1].0.0);
      assert_eq!(w[0].1.control_points.last(), w[1].1.control_points.first());
      assert_eq!(w[0].1.end(), f(w[0].0.1));
    }
    for (_, c) in pieces {
      assert!(c.is_polynomial());
      assert!(c.control_points.iter().all(|p| p.z == 1.));
    }
  }

  #[test]
  fn fits_circle_within_tolerance() {
    let f = |s: f64| Vec2::new(s.cos(), s.sin());
    // The piece count bounds guard the fit quality (an optimal cubic needs 8 pieces for 1e-6).
    for (degree, tol, max_pieces) in [(3, 1e-6, 8), (3, 1e-9, 32), (2, 1e-5, 32), (5, 1e-10, 16)] {
      let range = (0.1, 0.1 + 1.5 * PI);
      let pieces = fit_bezier_2d(&f, range, degree, &circle_error, tol, 20).unwrap();
      check_chain(&pieces, &f, range);
      assert!(pieces.len() <= max_pieces);
      for (r, c) in &pieces {
        assert_eq!(c.degree(), degree);
        assert!(circle_error(c, *r) <= tol);
      }
    }
  }

  #[test]
  fn cubic_target_is_one_piece() {
    let target = RationalBezierCurve2::from_euclid(
      &[
        Vec2::new(0., 0.),
        Vec2::new(1., 2.),
        Vec2::new(3., 2.),
        Vec2::new(4., 0.),
      ],
      None,
    );
    let f = |s: f64| target.eval(s);
    let error = |c: &RationalBezierCurve2, r: (f64, f64)| {
      (0..=50)
        .map(|i| {
          let t = i as f64 / 50.;
          let guess = r.0 + (r.1 - r.0) * t;
          closest_point_on_curve(&target, c.eval(t), Some(guess)).1
        })
        .fold(0., f64::max)
    };
    let pieces = fit_bezier_2d(&f, (0., 1.), 3, &error, 1e-10, 10).unwrap();
    assert_eq!(pieces.len(), 1);
    check_chain(&pieces, &f, (0., 1.));
  }

  #[test]
  fn lines_and_reversed_ranges() {
    let f = |s: f64| Vec2::new(2. * s + 1., -s);
    let error = |c: &RationalBezierCurve2, _: (f64, f64)| {
      (0..=10)
        .map(|i| {
          let p = c.eval(i as f64 / 10.);
          (p.y + (p.x - 1.) / 2.).abs()
        })
        .fold(0., f64::max)
    };
    let pieces = fit_bezier_2d(&f, (3., -1.), 1, &error, 1e-12, 4).unwrap();
    assert_eq!(pieces.len(), 1);
    check_chain(&pieces, &f, (3., -1.));
    let pieces = fit_bezier_2d(&f, (3., -1.), 4, &error, 1e-12, 4).unwrap();
    check_chain(&pieces, &f, (3., -1.));
  }

  #[test]
  fn degenerate_target() {
    let f = |_: f64| Vec2::new(1., 1.);
    let error =
      |c: &RationalBezierCurve2, _: (f64, f64)| (c.eval(0.5) - Vec2::new(1., 1.)).length();
    let pieces = fit_bezier_2d(&f, (0., 1.), 3, &error, 1e-12, 4).unwrap();
    assert_eq!(pieces.len(), 1);
  }

  #[test]
  fn failures() {
    let f = |s: f64| Vec2::new(s.cos(), s.sin());
    let err = fit_bezier_2d(&f, (0., 3.), 3, &circle_error, -1., 3).unwrap_err();
    assert_eq!(err.kind, crate::ErrorKind::Numerical);
    let nan = |s: f64| Vec2::new(if s > 0.5 { f64::NAN } else { s }, 0.);
    assert!(fit_bezier_2d(&nan, (0., 1.), 3, &circle_error, 1., 3).is_err());
    assert!(fit_bezier_2d(&f, (0., 1.), 0, &circle_error, 1., 3).is_err());
    assert!(fit_bezier_2d(&f, (0., f64::INFINITY), 3, &circle_error, 1., 3).is_err());
  }

  #[test]
  fn basis_partition_of_unity() {
    for n in 1..8 {
      for i in 0..=10 {
        let b = bernstein_basis(n, i as f64 / 10.);
        assert!((b.iter().sum::<f64>() - 1.).abs() < 1e-15);
      }
    }
  }
}
