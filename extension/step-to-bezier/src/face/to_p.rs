//! Mapping native loops into the face P space. Pole segments and iso-lines become straight
//! lines, exact native curves are split at the map breakpoints and mapped by the per segment
//! affine maps, everything else is fitted with the error measured in model space.

use super::*;

pub(super) struct LoopMapper<'a> {
  fc: &'a FaceCtx<'a>,
  space: &'a SurfaceParamSpace,
  p_period: [Option<f64>; 2],
  pub fitted_curve_count: usize,
}

/// Mapped pieces of one curve with their exactness.
type Pieces = Vec<(RationalBezierCurve2, bool)>;

impl<'a> LoopMapper<'a> {
  pub fn new(
    fc: &'a FaceCtx<'a>,
    space: &'a SurfaceParamSpace,
    p_period: [Option<f64>; 2],
  ) -> Self {
    Self {
      fc,
      space,
      p_period,
      fitted_curve_count: 0,
    }
  }

  /// Every curve end is snapped to the P position of its shared vertex, so the loop is
  /// connected bitwise; a band loop ends exactly at `start + winding · p_period` (computed the
  /// way `trim` checks it).
  pub fn map_loop(&mut self, l: &NativeLoop) -> ConvertResult<PLoop> {
    let starts: Vec<_> = l
      .curves
      .iter()
      .map(|c| self.snap_to_lines(self.space.to_p(c.start())))
      .collect();
    let mut offset = Vec2::new(0., 0.);
    for axis in 0..2 {
      let w = l.winding[axis];
      if w == 0 {
        continue;
      }
      let n = self.p_period[axis].ok_or_else(|| {
        ConvertError::topology(
          None,
          format!("loop winds {w} times around native axis {axis}, but the P map is not periodic"),
        )
      })?;
      set_coord(&mut offset, axis, w as f64 * n);
    }

    let mut curves = Vec::with_capacity(l.curves.len());
    for (k, c) in l.curves.iter().enumerate() {
      let a = starts[k];
      let b = match starts.get(k + 1) {
        Some(b) => *b,
        None => starts[0] + offset,
      };
      let pieces = self.map_curve(c, a, b)?;
      let count = pieces.len();
      let mut prev_end = a;
      for (i, (piece, exact)) in pieces.into_iter().enumerate() {
        let end = if i + 1 == count { b } else { piece.end() };
        let curve = piece.normalize_end_weights().with_endpoints(prev_end, end);
        prev_end = end;
        let degenerate = curve
          .control_points
          .iter()
          .all(|p| p.to_euclid() == curve.start());
        if degenerate {
          continue;
        }
        for curve in fit_into_cell(curve) {
          curves.push(TrimCurve {
            curve,
            source: c.source,
            exact,
          });
        }
      }
    }
    Ok(PLoop {
      curves,
      winding: l.winding,
    })
  }

  fn map_curve(&mut self, c: &NativeCurve, a: Vec2<f64>, b: Vec2<f64>) -> ConvertResult<Pieces> {
    let line = RationalBezierCurve2::line(a, b);
    match &c.geom {
      NativeGeom::Line => Ok(vec![(line, true)]),
      _ if c.iso_exact => Ok(vec![(line, true)]),
      NativeGeom::Exact { exact, .. } if c.iso.is_some() => Ok(vec![(line, *exact)]),
      NativeGeom::Exact { segments, exact } => self.map_exact(segments, *exact),
      NativeGeom::Projected => {
        let range = (0., c.s_end());
        let near_iso = c.iso.is_some() || c.iso_dev.iter().any(|d| *d <= self.fc.ctx.fit_tol);
        if near_iso && self.projected_error(c, &line, range) <= self.fc.ctx.fit_tol {
          return Ok(vec![(line, false)]);
        }
        let pieces = self.fit_projected(c, a, b)?;
        self.fitted_curve_count += 1;
        Ok(pieces.into_iter().map(|q| (q, false)).collect())
      }
    }
  }

  /// Fit a projected curve, split first where it crosses a map breakpoint (the P curve has a
  /// tangent discontinuity there, and the split point is snapped exactly onto the cell
  /// boundary line) and at tangent discontinuities of the 3D chain.
  fn fit_projected(
    &self,
    c: &NativeCurve,
    a: Vec2<f64>,
    b: Vec2<f64>,
  ) -> ConvertResult<Vec<RationalBezierCurve2>> {
    let f = |s: f64| {
      c.native_at(self.fc, s)
        .map_or(Vec2::new(f64::NAN, f64::NAN), |uv| self.space.to_p(uv))
    };
    // Crossings at a curve end (a vertex on a cell line) are not split points.
    let at_end = |s: f64, axis: usize, p: f64| {
      let mut q = f(s);
      set_coord(&mut q, axis, p);
      self.close_3d(q, a) || self.close_3d(q, b)
    };
    let mut splits: Vec<(f64, Option<(usize, f64)>)> = self
      .break_crossings(c)
      .into_iter()
      .filter(|(s, axis, p)| !at_end(*s, *axis, *p))
      .map(|(s, axis, p)| (s, Some((axis, p))))
      .collect();
    splits.extend(chain_kinks(&c.chain3).into_iter().map(|s| (s, None)));
    splits.sort_by(|a, b| a.0.total_cmp(&b.0));
    let end = c.s_end();
    let min_gap = 1e-9 * end;
    let mut bounds = vec![(0., None)];
    for split in splits {
      let last = bounds[bounds.len() - 1].0;
      if split.0 - last > min_gap && end - split.0 > min_gap {
        bounds.push(split);
      }
    }
    bounds.push((end, None));

    let error = |q: &RationalBezierCurve2, r: (f64, f64)| self.projected_error(c, q, r);
    let mut out: Vec<RationalBezierCurve2> = Vec::new();
    for w in bounds.windows(2) {
      let ((s0, snap), (s1, _)) = (w[0], w[1]);
      let pieces = if chain_length(&c.chain3, s0, s1) <= self.fc.ctx.fit_tol {
        // Shorter than the tolerance: a line is within tolerance, and fitting would only
        // amplify the projection noise.
        vec![((s0, s1), RationalBezierCurve2::line(f(s0), f(s1)))]
      } else {
        fit_bezier_2d(
          &f,
          (s0, s1),
          self.fc.ctx.fit_degree,
          &error,
          self.fc.ctx.fit_tol,
          MAX_FIT_DEPTH,
        )
        .map_err(|e| self.curve_error(c, e))?
      };
      let first = out.len();
      out.extend(pieces.into_iter().map(|(_, q)| q));
      if let (Some((axis, p)), Some(prev)) = (snap, first.checked_sub(1)) {
        let mut joint = out[first].start();
        set_coord(&mut joint, axis, p);
        out[prev] = out[prev].with_endpoints(out[prev].start(), joint);
        out[first] = out[first].with_endpoints(joint, out[first].end());
      }
    }
    Ok(out)
  }

  /// Chain parameters where the projected curve crosses a breakpoint of the u or v map, with
  /// the crossed axis and the exact P coordinate of the breakpoint. Located from the samples,
  /// refined by regula falsi on the projection.
  fn break_crossings(&self, c: &NativeCurve) -> Vec<(f64, usize, f64)> {
    let maps = [&self.space.u_map, &self.space.v_map];
    // The end samples hold the resolved vertex positions, the fit target is the projection.
    let n = c.samples.len();
    let mut samples = c.samples.clone();
    for k in [0, n - 1] {
      if let Some(uv) = c.native_at(self.fc, samples[k].s) {
        samples[k].uv = uv;
      }
    }
    let mut out = Vec::new();
    for w in samples.windows(2) {
      let (a, b) = (w[0], w[1]);
      for (axis, map) in maps.iter().enumerate() {
        let (xa, xb) = (coord(a.uv, axis), coord(b.uv, axis));
        for value in map.breaks_between(xa, xb) {
          if let Some(s) = self.locate_crossing(c, axis, value, (a.s, xa), (b.s, xb)) {
            out.push((s, axis, map.to_p(value)));
          }
        }
      }
    }
    out
  }

  fn locate_crossing(
    &self,
    c: &NativeCurve,
    axis: usize,
    value: f64,
    a: (f64, f64),
    b: (f64, f64),
  ) -> Option<f64> {
    let eps = 1e-13 * (1. + value.abs());
    let (mut s0, mut g0) = (a.0, a.1 - value);
    let (mut s1, mut g1) = (b.0, b.1 - value);
    if g0.is_nan() || g1.is_nan() || g0 * g1 >= 0. {
      return None;
    }
    let mut side = 0;
    for _ in 0..MAX_CROSSING_ITERATIONS {
      let s = (s0 * g1 - s1 * g0) / (g1 - g0);
      if !(s > s0 && s < s1) {
        return Some(0.5 * (s0 + s1));
      }
      let g = coord(c.native_at(self.fc, s)?, axis) - value;
      if g.abs() <= eps || s1 - s0 <= 1e-15 * (1. + s.abs()) {
        return Some(s);
      }
      if g * g0 > 0. {
        (s0, g0) = (s, g);
        if side == -1 {
          g1 *= 0.5;
        }
        side = -1;
      } else {
        (s1, g1) = (s, g);
        if side == 1 {
          g0 *= 0.5;
        }
        side = 1;
      }
    }
    Some((s0 * g1 - s1 * g0) / (g1 - g0))
  }

  /// Split at the map breakpoints, map affine pieces exactly, fit the pieces lying in non
  /// affine (circular arc) segments.
  fn map_exact(&mut self, segments: &[RationalBezierCurve2], exact: bool) -> ConvertResult<Pieces> {
    let mut out = Vec::new();
    let mut fitted = false;
    for seg in segments {
      for piece_u in split_at_breaks(seg, &self.space.u_map, Axis::U) {
        for piece in split_at_breaks(&piece_u, &self.space.v_map, Axis::V) {
          let mid = piece.eval(0.5);
          let (_, au) = self.space.u_map.segment_affine(mid.x);
          let (_, av) = self.space.v_map.segment_affine(mid.y);
          if let (Some((su, ou)), Some((sv, ov))) = (au, av) {
            out.push((
              piece.affine_map(Vec2::new(su, sv), Vec2::new(ou, ov)),
              exact,
            ));
            continue;
          }
          let f = |t: f64| self.space.to_p(piece.eval(t));
          let error = |q: &RationalBezierCurve2, r: (f64, f64)| self.native_error(&piece, q, r);
          let pieces = fit_bezier_2d(
            &f,
            (0., 1.),
            self.fc.ctx.fit_degree,
            &error,
            self.fc.ctx.fit_tol,
            MAX_FIT_DEPTH,
          )?;
          out.extend(pieces.into_iter().map(|(_, q)| (q, false)));
          fitted = true;
        }
      }
    }
    if fitted {
      self.fitted_curve_count += 1;
    }
    Ok(out)
  }

  /// Two P points map to model space points closer than the snapping distance.
  fn close_3d(&self, p: Vec2<f64>, q: Vec2<f64>) -> bool {
    let surface = self.fc.surface;
    let d = surface.eval(self.space.to_native(p)) - surface.eval(self.space.to_native(q));
    d.length() <= SNAP_FACTOR * self.fc.ctx.fit_tol
  }

  /// Coordinates very close to a cell line (in P and in model space) are moved exactly onto
  /// it, so a vertex on a knot line is not seen on either side of it by `trim`.
  fn snap_to_lines(&self, p: Vec2<f64>) -> Vec2<f64> {
    let mut out = p;
    for axis in 0..2 {
      let x = coord(p, axis);
      let r = x.round();
      if x == r || (x - r).abs() > P_SNAP {
        continue;
      }
      let mut q = out;
      set_coord(&mut q, axis, r);
      if self.close_3d(q, out) {
        out = q;
      }
    }
    out
  }

  /// Max model space distance between the P curve `q` mapped onto the surface and the
  /// projected edge, over samples of `q` (`range` is the edge parameter range `q` stands
  /// for, giving the search guesses).
  fn projected_error(&self, c: &NativeCurve, q: &RationalBezierCurve2, range: (f64, f64)) -> f64 {
    if !monotone(q) {
      return f64::INFINITY;
    }
    let surface = self.fc.surface;
    (0..=ERROR_SAMPLES)
      .map(|k| {
        let t = k as f64 / ERROR_SAMPLES as f64;
        let x = surface.eval(self.space.to_native(q.eval(t)));
        let guess = range.0 + (range.1 - range.0) * t;
        let s = closest_on_chain_near(&c.chain3, x, guess);
        match c.native_at(self.fc, s) {
          Some(uv) => (x - surface.eval(uv)).length(),
          None => f64::INFINITY,
        }
      })
      .fold(0., |m, d| if d.is_nan() { f64::INFINITY } else { m.max(d) })
  }

  /// Max model space distance between the P curve `q` and the exact native curve `target`
  /// (through the closest native point).
  fn native_error(
    &self,
    target: &RationalBezierCurve2,
    q: &RationalBezierCurve2,
    range: (f64, f64),
  ) -> f64 {
    if !monotone(q) {
      return f64::INFINITY;
    }
    let surface = self.fc.surface;
    (0..=ERROR_SAMPLES)
      .map(|k| {
        let t = k as f64 / ERROR_SAMPLES as f64;
        let native = self.space.to_native(q.eval(t));
        let guess = range.0 + (range.1 - range.0) * t;
        let (s, _) = closest_point_on_curve(target, native, Some(guess));
        (surface.eval(native) - surface.eval(target.eval(s))).length()
      })
      .fold(0., |m, d| if d.is_nan() { f64::INFINITY } else { m.max(d) })
  }

  fn curve_error(&self, c: &NativeCurve, e: ConvertError) -> ConvertError {
    match c.source {
      TrimCurveSource::Edge { edge, .. } => match self.fc.model.edges.get(edge.index()) {
        Some(edge) => e.at(edge.step_id),
        None => e,
      },
      _ => e,
    }
  }
}

const MAX_FIT_DEPTH: usize = 14;
/// Max P space distance (cells have unit size) a vertex is moved to put it on a cell line.
const P_SNAP: f64 = 1e-6;
/// Model space snapping distance in `fit_tol` units.
const SNAP_FACTOR: f64 = 1e-3;
const MAX_CROSSING_ITERATIONS: usize = 40;
/// Joints of a 3D chain whose tangent directions differ by more than this (cosine) are kinks.
const KINK_COS: f64 = 0.9999;
const ERROR_SAMPLES: usize = 16;
/// Crossings closer than this (curve parameter) to a curve end are the end itself.
const SPLIT_T_EPS: f64 = 1e-12;

/// Chain parameter (segment index + t) of the point of `chain` closest to `p`, searched in
/// the segment of `guess` and its neighbors.
fn closest_on_chain_near(chain: &[RationalBezierCurve3], p: Vec3<f64>, guess: f64) -> f64 {
  let n = chain.len();
  if n == 0 {
    return 0.;
  }
  let i = (guess.floor().max(0.) as usize).min(n - 1);
  let mut best = (f64::INFINITY, guess);
  let first = i.saturating_sub(1);
  for (j, segment) in chain.iter().enumerate().take(i + 2).skip(first) {
    let t_guess = match j.cmp(&i) {
      std::cmp::Ordering::Equal => guess - i as f64,
      std::cmp::Ordering::Less => 1.,
      std::cmp::Ordering::Greater => 0.,
    };
    let (t, d) = closest_point_on_curve(segment, p, Some(t_guess));
    if d < best.0 {
      best = (d, j as f64 + t);
    }
  }
  best.1
}

/// A curve inside one cell whose control polygon crosses the cell boundary is subdivided
/// (exactly, at parameter midpoints) until every control polygon is inside the cell, so the
/// pieces `trim` outputs keep their control points in `[0,1]²`.
fn fit_into_cell(curve: RationalBezierCurve2) -> Vec<RationalBezierCurve2> {
  if curve.degree() < 2 {
    return vec![curve];
  }
  let mid = curve.eval(0.5);
  let cell = Vec2::new(mid.x.floor(), mid.y.floor());
  let inside = |p: Vec2<f64>| {
    let d = p - cell;
    d.x >= -CELL_EPS && d.y >= -CELL_EPS && d.x <= 1. + CELL_EPS && d.y <= 1. + CELL_EPS
  };
  let hull_inside = |c: &RationalBezierCurve2| {
    let (lo, hi) = c.control_bbox();
    inside(lo) && inside(hi)
  };
  if hull_inside(&curve)
    || !(0..=CELL_SAMPLES).all(|k| inside(curve.eval(k as f64 / CELL_SAMPLES as f64)))
  {
    return vec![curve];
  }
  let mut out = Vec::new();
  let mut stack = vec![(curve, 0)];
  while let Some((c, depth)) = stack.pop() {
    if depth >= MAX_CELL_SUBDIVISION || hull_inside(&c) {
      out.push(c);
      continue;
    }
    let (left, right) = c.split(0.5);
    stack.push((right, depth + 1));
    stack.push((left, depth + 1));
  }
  out
}

const CELL_EPS: f64 = 1e-9;
const CELL_SAMPLES: usize = 32;
const MAX_CELL_SUBDIVISION: usize = 8;

/// Polyline length of the chain between the parameters `s0` and `s1`.
fn chain_length(chain: &[RationalBezierCurve3], s0: f64, s1: f64) -> f64 {
  const STEPS: usize = 8;
  let mut prev = chain_eval(chain, s0);
  let mut length = 0.;
  for k in 1..=STEPS {
    let p = chain_eval(chain, s0 + (s1 - s0) * k as f64 / STEPS as f64);
    length += (p - prev).length();
    prev = p;
  }
  length
}

/// Integer chain parameters where consecutive segments meet with a tangent discontinuity.
fn chain_kinks(chain: &[RationalBezierCurve3]) -> Vec<f64> {
  (1..chain.len())
    .filter(|i| {
      let (_, d0) = chain[i - 1].eval_with_derivative(1.);
      let (_, d1) = chain[*i].eval_with_derivative(0.);
      let cos = d0.dot(d1) / (d0.length() * d1.length());
      cos.is_nan() || cos < KINK_COS
    })
    .map(|i| i as f64)
    .collect()
}

/// The curve progresses along its chord: every control polygon leg has a non negative
/// component along the chord (for polynomial curves the hodograph then never points
/// backwards), which rules out folds and cusps in fitted pieces.
fn monotone(q: &RationalBezierCurve2) -> bool {
  let chord = q.end() - q.start();
  chord.length() > 0.
    && q
      .control_points
      .windows(2)
      .all(|w| (w[1].to_euclid() - w[0].to_euclid()).dot(chord) >= 0.)
}

/// Split `curve` where it crosses a breakpoint of `map` along `axis`.
fn split_at_breaks(
  curve: &RationalBezierCurve2,
  map: &PiecewiseParamMap,
  axis: Axis,
) -> Vec<RationalBezierCurve2> {
  let (lo, hi) = curve.control_bbox();
  let (lo, hi) = match axis {
    Axis::U => (lo.x, hi.x),
    Axis::V => (lo.y, hi.y),
  };
  let mut ts: Vec<_> = map
    .breaks_between(lo, hi)
    .into_iter()
    .flat_map(|b| axis_line_crossings(curve, axis, b, SPLIT_T_EPS))
    .filter(|t| *t > SPLIT_T_EPS && *t < 1. - SPLIT_T_EPS)
    .collect();
  if ts.is_empty() {
    return vec![curve.clone()];
  }
  ts.sort_by(f64::total_cmp);
  ts.dedup_by(|a, b| (*a - *b).abs() <= SPLIT_T_EPS);
  let mut out = Vec::with_capacity(ts.len() + 1);
  let mut t0 = 0.;
  for t in ts.into_iter().chain(std::iter::once(1.)) {
    out.push(curve.sub_curve(t0, t));
    t0 = t;
  }
  out
}
