//! Numerical closest point search shared by B-spline and swept surfaces.

use super::*;

/// Axis description for the numerical search: clamp interval of bounded axes and
/// `(start, period)` of periodic axes (used to pick the returned representative).
#[derive(Debug, Clone, Copy)]
pub struct SearchAxes {
  pub bounds: [Option<(f64, f64)>; 2],
  pub periodic: [Option<(f64, f64)>; 2],
}

/// Projection strategy of the numerical surfaces. With a guess a local refinement starts
/// there and is accepted when the result is within `tol`; otherwise (or without guess) every
/// seed from `seeds` is refined and the closest result wins. Periodic coordinates come back
/// as the representative nearest to the guess (without guess, inside the base period). A
/// point projecting onto a pole gets the pole coordinate exactly and keeps the guess for the
/// other coordinate.
pub fn project_numerically<S: SurfaceModel + ?Sized>(
  s: &S,
  axes: SearchAxes,
  tol: f64,
  p: Vec3<f64>,
  guess: Option<Vec2<f64>>,
  seeds: impl FnOnce() -> Vec<Vec2<f64>>,
) -> Option<SurfaceProjection> {
  if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
    return None;
  }
  let guess = guess.filter(|g| g.x.is_finite() && g.y.is_finite());
  let mut best = guess.map(|g| refine_projection(s, axes.bounds, p, g));
  if best.is_none_or(|b| b.1.is_nan() || b.1 > tol) {
    for seed in seeds() {
      let r = refine_projection(s, axes.bounds, p, seed);
      if best.is_none_or(|b| r.1 < b.1 || b.1.is_nan()) {
        best = Some(r);
      }
    }
  }
  let (native, _) = best?;
  let mut n = [native.x, native.y];
  let guess = guess.map(|g| [g.x, g.y]);
  for (i, periodic) in axes.periodic.iter().enumerate() {
    if let Some((start, period)) = *periodic {
      n[i] = match guess {
        Some(g) => nearest_representative(n[i], Some(period), Some(g[i])),
        None => start + (n[i] - start).rem_euclid(period),
      };
    }
  }
  let q = s.eval(Vec2::new(n[0], n[1]));
  for pole in s.poles() {
    if (q - pole.position).length() <= 1e-3 * tol {
      let (a, o) = (pole.axis.index(), pole.axis.other().index());
      n[a] = pole.value;
      if let Some(g) = guess {
        n[o] = g[o];
        if let Some((lo, hi)) = axes.bounds[o] {
          n[o] = n[o].clamp(lo, hi);
        }
      }
    }
  }
  let native = Vec2::new(n[0], n[1]);
  finite_projection(native, (s.eval(native) - p).length())
}

/// Levenberg-Marquardt minimization of `|S(x) - p|²` starting at `start`, coordinates clamped
/// to `bounds`. Every accepted step decreases the distance, the iteration count is bounded.
/// Returns the native position and the distance.
pub fn refine_projection<S: SurfaceModel + ?Sized>(
  s: &S,
  bounds: [Option<(f64, f64)>; 2],
  p: Vec3<f64>,
  start: Vec2<f64>,
) -> (Vec2<f64>, f64) {
  let clamp = |x: Vec2<f64>| {
    let c = |v: f64, b: Option<(f64, f64)>| b.map_or(v, |(lo, hi)| v.clamp(lo, hi));
    Vec2::new(c(x.x, bounds[0]), c(x.y, bounds[1]))
  };
  let mut x = clamp(start);
  let (mut q, mut su, mut sv) = s.eval_with_partials(x);
  let mut f = (q - p).dot(q - p);
  let mut lambda = 1e-9;
  for _ in 0..MAX_ITERATIONS {
    if f.is_nan() || f <= 0. {
      break;
    }
    let d = q - p;
    let g = Vec2::new(su.dot(d), sv.dot(d));
    let (a, b, c) = (su.dot(su), su.dot(sv), sv.dot(sv));
    let tiny = (a + c) * 1e-15 + f64::MIN_POSITIVE;
    let mut accepted = None;
    for _ in 0..MAX_DAMPING {
      let (a2, c2) = (a * (1. + lambda) + tiny, c * (1. + lambda) + tiny);
      let det = a2 * c2 - b * b;
      if det.is_nan() || det <= 0. {
        lambda = lambda * 10. + 1e-12;
        continue;
      }
      let step = Vec2::new(-(c2 * g.x - b * g.y) / det, -(a2 * g.y - b * g.x) / det);
      let next = clamp(x + step);
      if next == x || !(next.x.is_finite() && next.y.is_finite()) {
        break;
      }
      let (qn, sun, svn) = s.eval_with_partials(next);
      let f_next = (qn - p).dot(qn - p);
      if f_next < f {
        accepted = Some((next, qn, sun, svn, f_next));
        lambda *= 0.1;
        break;
      }
      lambda = lambda * 10. + 1e-12;
    }
    let Some((next, qn, sun, svn, f_next)) = accepted else {
      break;
    };
    let moved = next - x;
    (x, q, su, sv, f) = (next, qn, sun, svn, f_next);
    if moved.x.abs() <= 1e-15 * (1. + x.x.abs()) && moved.y.abs() <= 1e-15 * (1. + x.y.abs()) {
      break;
    }
  }
  (x, f.sqrt())
}

const MAX_ITERATIONS: usize = 200;
const MAX_DAMPING: usize = 40;

/// Seeds for the global search: the best samples of a grid over each patch, at most one per
/// patch and `count` overall. `native(patch, (a, b))` maps a patch local parameter to native.
pub fn patch_seeds(
  patches: &[&RationalBezierSurface],
  p: Vec3<f64>,
  count: usize,
  native: impl Fn(usize, Vec2<f64>) -> Vec2<f64>,
) -> Vec<Vec2<f64>> {
  let mut best: Vec<_> = patches
    .iter()
    .enumerate()
    .map(|(k, patch)| {
      let n = patch.degree_u.max(patch.degree_v) + 3;
      let mut local = (f64::INFINITY, Vec2::new(0.5, 0.5));
      for i in 0..=n {
        for j in 0..=n {
          let ab = Vec2::new(i as f64 / n as f64, j as f64 / n as f64);
          let d = (patch.eval(ab.x, ab.y) - p).length();
          if d < local.0 {
            local = (d, ab);
          }
        }
      }
      (local.0, native(k, local.1))
    })
    .collect();
  best.sort_by(|a, b| a.0.total_cmp(&b.0));
  best.into_iter().take(count).map(|b| b.1).collect()
}

pub fn finite_projection(native: Vec2<f64>, distance: f64) -> Option<SurfaceProjection> {
  (native.x.is_finite() && native.y.is_finite() && distance.is_finite())
    .then_some(SurfaceProjection { native, distance })
}
