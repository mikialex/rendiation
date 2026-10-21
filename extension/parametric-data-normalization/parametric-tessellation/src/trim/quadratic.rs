use std::ops::Range;

use parametric_normalized_model::*;
use rendiation_algebra::*;

/// Parameters of the conversion of trim curves to polynomial quadratic Béziers.
#[derive(Debug, Clone, Copy)]
pub struct TrimQuadraticConfig {
  /// Allowed deviation in model space. It is converted per patch to the parameter domain by
  /// dividing by the patch's parametric speed bound (`parametric_speed_bound`).
  pub tolerance_3d: f64,
  /// Clamp of the per patch parameter domain tolerance. The lower bound is about what f32
  /// resolves in `[0, 1]`, the upper bound keeps tiny patches from getting very coarse trims.
  pub min_uv_tolerance: f64,
  pub max_uv_tolerance: f64,
}

impl TrimQuadraticConfig {
  /// `tolerance_3d` is the model tolerance.
  pub fn for_model(model: &ParametricNormalizedModel) -> Self {
    Self {
      tolerance_3d: model.tolerance,
      min_uv_tolerance: 1e-6,
      max_uv_tolerance: 1e-3,
    }
  }

  pub fn uv_tolerance(&self, surface: &RationalBezierSurface) -> f64 {
    let speed = parametric_speed_bound(surface);
    let tolerance = self.tolerance_3d / speed;
    if tolerance.is_nan() {
      return self.min_uv_tolerance;
    }
    tolerance.clamp(self.min_uv_tolerance, self.max_uv_tolerance)
  }
}

/// A polynomial quadratic Bézier piece of a trim loop in the patch domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimQuadratic {
  pub points: [Vec2<f64>; 3],
}

impl TrimQuadratic {
  pub fn eval(&self, t: f64) -> Vec2<f64> {
    let [p0, p1, p2] = self.points;
    let s = 1. - t;
    p0 * (s * s) + p1 * (2. * s * t) + p2 * (t * t)
  }
}

/// The trim loops of one patch as chains of quadratics.
#[derive(Debug, Clone)]
pub struct PatchTrimQuadratics {
  /// The segments of all loops, a loop is a consecutive range in which every segment starts
  /// exactly (bitwise) where the previous one ends, and the last ends where the first starts.
  pub segments: Vec<TrimQuadratic>,
  pub loops: Vec<Range<usize>>,
  /// Parameter domain tolerance used for this patch.
  pub uv_tolerance: f64,
  /// Pieces that still exceeded the tolerance at the subdivision limit.
  pub failures: usize,
}

/// Convert the trim loops of `patch`. Curves collapsed to a point are dropped (loops stay
/// closed). `Pole` curves are kept: they are degenerate in model space, not in the parameter
/// domain, where they are a boundary segment of the loop. Lines get the midpoint as middle control point, polynomial quadratics
/// are kept, polynomial cubics are split uniformly into the number of pieces given by the
/// closed form error bound, other curves (rational, higher degree) are subdivided recursively
/// with a sampled error check. Split points are evaluated once and shared by both pieces.
pub fn trim_quadratics(
  patch: &TrimmedBezierPatch,
  config: &TrimQuadraticConfig,
) -> PatchTrimQuadratics {
  let uv_tolerance = config.uv_tolerance(&patch.surface);
  let mut result = PatchTrimQuadratics {
    segments: Vec::new(),
    loops: Vec::new(),
    uv_tolerance,
    failures: 0,
  };
  for trim_loop in &patch.loops {
    let start = result.segments.len();
    for curve in &trim_loop.curves {
      result.failures += convert_curve(&curve.curve, uv_tolerance, &mut result.segments);
    }
    if result.segments.len() > start {
      result.loops.push(start..result.segments.len());
    }
  }
  result
}

/// Upper bound of `|∂S/∂u|` and `|∂S/∂v|` over the patch. With `G = H - c·w` for the control
/// box center `c`, `∂S = (G' - (S - c)·w') / w`, bounded by the hodograph control nets,
/// `|S - c| ≤ radius` (convex hull) and `w ≥ w_min`.
pub fn parametric_speed_bound(s: &RationalBezierSurface) -> f64 {
  let (n, m) = (s.degree_u, s.degree_v);
  let h = |i: usize, j: usize| s.control_point(i, j);
  let euclid: Vec<_> = s.control_points.iter().map(|p| p.to_euclid()).collect();
  let mut min = euclid[0];
  let mut max = euclid[0];
  for p in &euclid {
    min = min.min(*p);
    max = max.max(*p);
  }
  let center = (min + max) * 0.5;
  let radius = euclid
    .iter()
    .map(|p| (*p - center).length())
    .fold(0., f64::max);
  let w_min = s
    .control_points
    .iter()
    .map(|p| p.w)
    .fold(f64::INFINITY, f64::min);
  if w_min.is_nan() || w_min <= 0. {
    return f64::INFINITY;
  }
  let g = |i: usize, j: usize| {
    let p = h(i, j);
    p.vector_part() - center * p.w
  };
  let mut speed: f64 = 0.;
  if n > 0 {
    let mut gu: f64 = 0.;
    let mut du: f64 = 0.;
    for j in 0..=m {
      for i in 1..=n {
        gu = gu.max((g(i, j) - g(i - 1, j)).length());
        du = du.max((h(i, j).w - h(i - 1, j).w).abs());
      }
    }
    speed = speed.max(n as f64 * (gu + du * radius) / w_min);
  }
  if m > 0 {
    let mut gv: f64 = 0.;
    let mut dv: f64 = 0.;
    for j in 1..=m {
      for i in 0..=n {
        gv = gv.max((g(i, j) - g(i, j - 1)).length());
        dv = dv.max((h(i, j).w - h(i, j - 1).w).abs());
      }
    }
    speed = speed.max(m as f64 * (gv + dv * radius) / w_min);
  }
  speed
}

/// The maximum distance between a polynomial cubic and the quadratic with the same end points
/// and middle control point `(3(P1 + P2) - P0 - P3) / 4` is `√3/36 · |P3 - 3P2 + 3P1 - P0|`
/// (at corresponding parameters, so also a bound of the Hausdorff distance). A piece of
/// parameter length `1/n` has its third difference scaled by `1/n³`.
pub fn cubic_piece_count(points: [Vec2<f64>; 4], tolerance: f64) -> usize {
  let [p0, p1, p2, p3] = points;
  let third = (p3 - p2 * 3. + p1 * 3. - p0).length();
  let error = 3f64.sqrt() / 36. * third;
  let n = (error / tolerance).cbrt().ceil();
  if n.is_nan() {
    return 1;
  }
  (n as usize).clamp(1, MAX_CUBIC_PIECES)
}

const MAX_CUBIC_PIECES: usize = 1 << 16;
const MAX_DEPTH: u32 = 20;
const ERROR_SAMPLES: usize = 32;

/// Returns the number of pieces exceeding the tolerance.
fn convert_curve(c: &RationalBezierCurve2, tolerance: f64, out: &mut Vec<TrimQuadratic>) -> usize {
  let (start, end) = (c.start(), c.end());
  let points: Vec<_> = c.control_points.iter().map(|p| p.to_euclid()).collect();
  if points.iter().all(|p| *p == start) {
    return 0;
  }
  match c.degree() {
    1 => {
      out.push(TrimQuadratic {
        points: [start, (start + end) * 0.5, end],
      });
      0
    }
    2 if c.is_polynomial() => {
      out.push(TrimQuadratic {
        points: [start, points[1], end],
      });
      0
    }
    3 if c.is_polynomial() => {
      let n = cubic_piece_count([points[0], points[1], points[2], points[3]], tolerance);
      let at = |i: usize| match i {
        0 => start,
        i if i == n => end,
        i => c.eval(i as f64 / n as f64),
      };
      let mut from = start;
      for i in 0..n {
        let to = at(i + 1);
        let piece = c.sub_curve(i as f64 / n as f64, (i + 1) as f64 / n as f64);
        let q: Vec<_> = piece.control_points.iter().map(|p| p.to_euclid()).collect();
        let middle = ((q[1] + q[2]) * 3. - q[0] - q[3]) * 0.25;
        out.push(TrimQuadratic {
          points: [from, middle, to],
        });
        from = to;
      }
      0
    }
    _ => subdivide(c, (0., start), (1., end), tolerance, 0, out),
  }
}

/// The quadratic through both ends and the curve point at the middle parameter.
fn subdivide(
  c: &RationalBezierCurve2,
  (t0, p0): (f64, Vec2<f64>),
  (t1, p2): (f64, Vec2<f64>),
  tolerance: f64,
  depth: u32,
  out: &mut Vec<TrimQuadratic>,
) -> usize {
  let tm = (t0 + t1) * 0.5;
  let pm = c.eval(tm);
  let q = TrimQuadratic {
    points: [p0, pm * 2. - (p0 + p2) * 0.5, p2],
  };
  let error = (1..ERROR_SAMPLES)
    .map(|i| {
      let s = i as f64 / ERROR_SAMPLES as f64;
      (c.eval(t0 + (t1 - t0) * s) - q.eval(s)).length()
    })
    .fold(0., f64::max);
  if error <= tolerance {
    out.push(q);
    return 0;
  }
  if depth >= MAX_DEPTH {
    out.push(q);
    return 1;
  }
  subdivide(c, (t0, p0), (tm, pm), tolerance, depth + 1, out)
    + subdivide(c, (tm, pm), (t1, p2), tolerance, depth + 1, out)
}
