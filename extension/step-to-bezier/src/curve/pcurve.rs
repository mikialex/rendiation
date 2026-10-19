//! Edge pcurves restricted to the edge, in the surface native uv space.
//!
//! The restriction is found in 3D: the pcurve parameters of the edge end points minimize
//! `|S(c(s)) - vertex|` (sampling then golden section refinement), which is immune to the
//! periodic ambiguity of uv projection. Where several intervals are possible (closed edges,
//! pcurves that repeat modulo a surface period) the candidate whose image follows `edge3` best
//! (distance at interior points) wins.

use std::cmp::Ordering;

use super::chain::*;
use super::edge3::VERTEX_DISTANCE_FACTOR;
use super::resolve::*;
use super::*;

pub(super) fn build(
  model: &BrepModel,
  edge_id: EdgeId,
  pcurve: Curve2Id,
  surface: &dyn SurfaceModel,
  edge3: &Edge3,
  ctx: &ConvertCtx,
) -> ConvertResult<Edge2> {
  let edge = model
    .edges
    .get(edge_id.index())
    .ok_or_else(|| ConvertError::invalid(None, format!("edge id {} out of range", edge_id.0)))?;
  build_pcurve(model, pcurve, surface, edge3, ctx).map_err(|e| e.at(edge.step_id))
}

fn build_pcurve(
  model: &BrepModel,
  pcurve: Curve2Id,
  surface: &dyn SurfaceModel,
  edge3: &Edge3,
  ctx: &ConvertCtx,
) -> ConvertResult<Edge2> {
  let (Some(first), Some(last)) = (edge3.segments.first(), edge3.segments.last()) else {
    // Degenerate (zero length) edge: nothing to restrict to.
    return Ok(Edge2 {
      segments: Vec::new(),
      iso: None,
    });
  };
  let (start, end) = (first.start(), last.end());
  let resolved = resolve(&Curves2(model), pcurve.index(), None)?;
  let curve = UvCurve::new(resolved, surface, ctx);
  let max_distance = VERTEX_DISTANCE_FACTOR * ctx.tol;
  let check = |d: f64, which: &str| {
    if d.is_nan() || d > max_distance {
      Err(ConvertError::invalid(
        None,
        format!(
          "pcurve image is {d} away from the edge {which} vertex (tol {})",
          ctx.tol
        ),
      ))
    } else {
      Ok(())
    }
  };

  let (s0, d0) = curve.locate(surface, start)?;
  check(d0, "start")?;
  let full = edge3.closed || (end - start).length() <= ctx.tol;
  let mut candidates = Vec::new();
  match (curve.period, &curve.resolved) {
    (Some(length), _) if full => {
      candidates.push((s0, s0 + length));
      candidates.push((s0, s0 - length));
    }
    (None, Resolved::Chain(chain)) if full => {
      // Image closed without a uv period (for example through a pole): the whole pcurve.
      let (a, b) = chain.domain();
      check((surface.eval(curve.eval(a)) - start).length(), "start")?;
      check((surface.eval(curve.eval(b)) - start).length(), "start")?;
      candidates.push((a, b));
      candidates.push((b, a));
    }
    _ if full => {
      return Err(ConvertError::invalid(
        None,
        "closed edge with a pcurve whose image is not closed",
      ));
    }
    (Some(length), _) => {
      let f = |s: f64| (surface.eval(curve.eval(s)) - end).length();
      let (s1, d1) = minimize_global(&f, s0, s0 + length, curve.samples());
      check(d1, "end")?;
      let forward = s0 + (s1 - s0).rem_euclid(length);
      candidates.push((s0, forward));
      candidates.push((s0, forward - length));
    }
    (None, _) => {
      let (s1, d1) = curve.locate(surface, end)?;
      check(d1, "end")?;
      candidates.push((s0, s1));
    }
  }

  // Interior points of the image must lie on `edge3` and progress along it in order (on a
  // closed edge both directions lie on it, only the order tells them apart).
  let score = |(a, b): (f64, f64)| {
    let mut distance = 0f64;
    let mut ordered = true;
    let mut previous = f64::NEG_INFINITY;
    for f in [0.25, 0.5, 0.75] {
      let p = surface.eval(curve.eval(a + (b - a) * f));
      let Some((i, t, d)) = closest_point_on_chain(&edge3.segments, p) else {
        return (false, f64::INFINITY);
      };
      let position = i as f64 + t;
      ordered &= position > previous;
      previous = position;
      distance = distance.max(d);
    }
    (ordered, distance)
  };
  let (best, (_, best_score)) = candidates
    .into_iter()
    .filter(|(a, b)| a != b)
    .map(|c| (c, score(c)))
    .min_by(|x, y| {
      (!x.1.0, x.1.1)
        .partial_cmp(&(!y.1.0, y.1.1))
        .unwrap_or(Ordering::Equal)
    })
    .ok_or_else(|| ConvertError::invalid(None, "pcurve restriction to the edge is empty"))?;
  if best_score.is_nan() || best_score > max_distance {
    return Err(ConvertError::invalid(
      None,
      format!(
        "pcurve image deviates {best_score} from the edge curve (tol {})",
        ctx.tol
      ),
    ));
  }

  let (a, b) = best;
  let mut segments = curve.resolved.restrict(a.min(b), a.max(b)).curves();
  if a > b {
    segments.reverse();
    for s in &mut segments {
      *s = s.reversed();
    }
  }
  let iso = detect_iso(&mut segments);
  Ok(Edge2 { segments, iso })
}

/// A resolved pcurve with its repetition length: after `period` the uv curve repeats, shifted
/// by a surface period vector (zero for curves closed in uv).
struct UvCurve {
  resolved: Resolved<Vec3<f64>>,
  period: Option<f64>,
  surface_period: [Option<f64>; 2],
}

impl UvCurve {
  fn new(mut resolved: Resolved<Vec3<f64>>, surface: &dyn SurfaceModel, ctx: &ConvertCtx) -> Self {
    let surface_period = surface.period();
    let period = match &mut resolved {
      Resolved::Line { vector, .. } => line_period(*vector, surface_period),
      Resolved::Chain(chain) => {
        if chain.period.is_none()
          && let Some(shift) = image_closed_shift(chain, surface, ctx)
        {
          chain.period = Some(Period {
            length: chain.length(),
            shift,
          });
        }
        chain.period.map(|p| p.length)
      }
    };
    Self {
      resolved,
      period,
      surface_period,
    }
  }

  fn eval(&self, s: f64) -> Vec2<f64> {
    self.resolved.eval(s)
  }

  fn samples(&self) -> usize {
    match &self.resolved {
      Resolved::Line { .. } => 64,
      Resolved::Chain(chain) => (16 * chain.pieces.len()).clamp(64, 4096),
    }
  }

  /// Parameter minimizing the 3D distance to `p`, with the distance.
  fn locate(&self, surface: &dyn SurfaceModel, p: Vec3<f64>) -> ConvertResult<(f64, f64)> {
    let f = |s: f64| (surface.eval(self.eval(s)) - p).length();
    match &self.resolved {
      Resolved::Chain(chain) => {
        let (a, b) = chain.domain();
        Ok(minimize_global(&f, a, b, self.samples()))
      }
      Resolved::Line { origin, vector } => {
        // Unbounded: start from the uv projection (any periodic representative), pick the
        // representative closest to the line, then minimize in 3D.
        let guess = surface
          .project(p, None)
          .ok_or_else(|| ConvertError::numerical(None, "surface projection failed"))?
          .native;
        let s = line_guess(*origin, *vector, guess, self.surface_period);
        let scale = 1. + guess.x.abs().max(guess.y.abs());
        Ok(minimize_local(&f, s, 1e-4 * scale / vector.length()))
      }
    }
  }
}

/// A uv line repeats on the surface when it runs along a periodic axis.
fn line_period(vector: Vec2<f64>, surface_period: [Option<f64>; 2]) -> Option<f64> {
  let tight = 1e-12 * vector.length();
  match surface_period {
    [Some(t), _] if vector.y.abs() <= tight => Some(t / vector.x.abs()),
    [_, Some(t)] if vector.x.abs() <= tight => Some(t / vector.y.abs()),
    _ => None,
  }
}

/// When the image of an open uv chain is closed and its uv end points differ by a surface
/// period vector, that vector (`None` when it is zero).
fn image_closed_shift(
  chain: &Chain<Vec3<f64>>,
  surface: &dyn SurfaceModel,
  ctx: &ConvertCtx,
) -> Option<Option<Vec2<f64>>> {
  let (a, b) = (chain.start_point(), chain.end_point());
  if (surface.eval(a) - surface.eval(b)).length() > VERTEX_DISTANCE_FACTOR * ctx.tol {
    return None;
  }
  let delta = b - a;
  let scale = chain.extent().max(a.x.abs()).max(a.y.abs());
  let mut shift = [0.; 2];
  for (axis, d) in [delta.x, delta.y].into_iter().enumerate() {
    shift[axis] = match surface.period()[axis] {
      Some(t) => {
        let k = (d / t).round();
        if (d - k * t).abs() > 1e-6 * t {
          return None;
        }
        k * t
      }
      None => {
        if d.abs() > 1e-6 * scale {
          return None;
        }
        0.
      }
    };
  }
  Some((shift != [0.; 2]).then(|| Vec2::new(shift[0], shift[1])))
}

/// Line parameter of the periodic representative of `guess` closest to the line; among
/// equally close ones (shifts along the line) the one with the smallest `|s|`.
fn line_guess(
  origin: Vec2<f64>,
  vector: Vec2<f64>,
  guess: Vec2<f64>,
  surface_period: [Option<f64>; 2],
) -> f64 {
  let vv = vector.dot(vector);
  let eval = |g: Vec2<f64>| {
    let s = (g - origin).dot(vector) / vv;
    (s, (origin + vector * s - g).length())
  };
  // Best period index per axis, solved on the affine perpendicular residual.
  let mut center = [0.; 2];
  for axis in 0..2 {
    let Some(t) = surface_period[axis] else {
      continue;
    };
    let e = if axis == 0 {
      Vec2::new(t, 0.)
    } else {
      Vec2::new(0., t)
    };
    let w = e - vector * (e.dot(vector) / vv);
    let base = guess + Vec2::new(center[0], center[1]);
    let (s, _) = eval(base);
    let r = base - (origin + vector * s);
    let k = if w.length() > 1e-9 * t {
      -r.dot(w) / w.dot(w)
    } else {
      // Shifts move along the line: smallest |s|.
      -s / (e.dot(vector) / vv)
    };
    center[axis] = k.round() * t;
  }
  let mut best = (f64::INFINITY, f64::INFINITY, 0.);
  let ks = [-1., 0., 1.];
  for ku in ks {
    for kv in ks {
      let du = surface_period[0].map_or(0., |t| center[0] + ku * t);
      let dv = surface_period[1].map_or(0., |t| center[1] + kv * t);
      let (s, perp) = eval(guess + Vec2::new(du, dv));
      let tie = 1e-9 * (1. + perp.max(best.0));
      if perp < best.0 - tie || (perp <= best.0 + tie && s.abs() < best.1) {
        best = (perp, s.abs(), s);
      }
    }
  }
  best.2
}

/// Global minimum of `f` on `[lo, hi]`: uniform sampling, then golden section refinement of
/// the best sampled local minima. Returns `(x, f(x))`.
fn minimize_global(f: &impl Fn(f64) -> f64, lo: f64, hi: f64, samples: usize) -> (f64, f64) {
  let n = samples.max(2);
  let xs: Vec<_> = (0..=n)
    .map(|i| match i {
      0 => lo,
      _ if i == n => hi,
      _ => lo + (hi - lo) * (i as f64 / n as f64),
    })
    .collect();
  let fs: Vec<_> = xs.iter().map(|x| f(*x)).collect();
  let mut minima: Vec<_> = (0..=n)
    .filter(|&i| {
      let left = if i > 0 { fs[i - 1] } else { f64::INFINITY };
      let right = if i < n { fs[i + 1] } else { f64::INFINITY };
      fs[i] <= left && fs[i] <= right
    })
    .collect();
  minima.sort_by(|a, b| fs[*a].total_cmp(&fs[*b]));
  let mut best = (xs[0], fs[0]);
  for &i in minima.iter().take(4) {
    let (a, b) = (xs[i.saturating_sub(1)], xs[(i + 1).min(n)]);
    let r = golden(f, a, b);
    if r.1 < best.1 || best.1.is_nan() {
      best = r;
    }
    if fs[i] < best.1 {
      best = (xs[i], fs[i]);
    }
  }
  best
}

/// Local minimum near `x`: golden section in a bracket that doubles while the minimum sits on
/// its boundary.
fn minimize_local(f: &impl Fn(f64) -> f64, x: f64, h: f64) -> (f64, f64) {
  let mut h = if h.is_finite() && h > 0. { h } else { 1e-6 };
  let mut best = (x, f(x));
  for _ in 0..80 {
    let (a, b) = (x - h, x + h);
    let r = golden(f, a, b);
    if r.1 < best.1 {
      best = r;
    }
    let margin = 1e-3 * h;
    if r.0 - a > margin && b - r.0 > margin {
      return best;
    }
    h *= 2.;
  }
  best
}

/// Golden section search on `[a, b]`, converging to a local minimum (or a bracket end).
fn golden(f: &impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> (f64, f64) {
  let ratio = 0.5 * (5f64.sqrt() - 1.);
  let mut x1 = b - ratio * (b - a);
  let mut x2 = a + ratio * (b - a);
  let (mut f1, mut f2) = (f(x1), f(x2));
  for _ in 0..200 {
    if b - a <= 4. * f64::EPSILON * (a.abs() + b.abs()).max(f64::MIN_POSITIVE) {
      break;
    }
    if f1 <= f2 {
      b = x2;
      x2 = x1;
      f2 = f1;
      x1 = b - ratio * (b - a);
      f1 = f(x1);
    } else {
      a = x1;
      x1 = x2;
      f1 = f2;
      x2 = a + ratio * (b - a);
      f2 = f(x2);
    }
  }
  let candidates = [(a, f(a)), (b, f(b)), (x1, f1), (x2, f2)];
  candidates
    .into_iter()
    .min_by(|x, y| x.1.total_cmp(&y.1))
    .unwrap_or((a, f(a)))
}

/// Relative tolerance on uv coordinates for iso-line detection.
const ISO_REL: f64 = 1e-11;

/// Detect an iso-line (all control points share `u` or `v`) and make the shared coordinate
/// bitwise identical in every control point.
fn detect_iso(segments: &mut [RationalBezierCurve2]) -> Option<IsoLine> {
  let points: Vec<_> = segments
    .iter()
    .flat_map(|s| s.control_points.iter().map(|p| p.to_euclid()))
    .collect();
  let first = *points.first()?;
  let scale = points
    .iter()
    .map(|p| p.x.abs().max(p.y.abs()))
    .fold(0., f64::max);
  let eps = ISO_REL * scale;
  let shared = |c: fn(&Vec2<f64>) -> f64| {
    let v = c(&first);
    points.iter().all(|p| (c(p) - v).abs() <= eps).then_some(v)
  };
  if let Some(u) = shared(|p| p.x) {
    for p in segments.iter_mut().flat_map(|s| &mut s.control_points) {
      p.x = u * p.z;
    }
    return Some(IsoLine::U(u));
  }
  if let Some(v) = shared(|p| p.y) {
    for p in segments.iter_mut().flat_map(|s| &mut s.control_points) {
      p.y = v * p.z;
    }
    return Some(IsoLine::V(v));
  }
  None
}
