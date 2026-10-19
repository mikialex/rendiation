//! Edge uses as curves in the surface native uv space.
//!
//! Three sources, in order of preference: the edge pcurve on the face surface (exact), the
//! exact affine projection of the 3D chain onto a plane, and the projection of the 3D chain
//! with continuation (adaptive samples, every projection seeded with the previous result).
//! Every curve carries dense native samples (bounding box, area, iso detection, projection
//! guesses) and its 3D chain (fitting error).

use super::iso::*;
use super::*;

#[derive(Clone)]
pub(super) struct NativeCurve {
  pub source: TrimCurveSource,
  pub geom: NativeGeom,
  /// 3D geometry in curve direction, empty for pole segments.
  pub chain3: Vec<RationalBezierCurve3>,
  /// Native samples in curve order, `s` from 0 to `s_end()`. After assembly the first and last
  /// samples hold the resolved vertex positions shared with the neighbor curves.
  pub samples: Vec<Sample>,
  /// Whole period translation applied to the native geometry since it was built.
  pub shift: Vec2<f64>,
  /// Iso-line axis (the coordinate that stays constant), when detected or declared.
  pub iso: Option<usize>,
  /// The iso property is guaranteed (pcurve iso, analytic proof from the 3D curve hint).
  pub iso_exact: bool,
  /// Max 3D effect of the variation of each native coordinate along the samples.
  pub iso_dev: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Sample {
  pub s: f64,
  pub uv: Vec2<f64>,
  pub pole: bool,
}

#[derive(Clone)]
pub(super) enum NativeGeom {
  /// Exact native curve chain (`shift` included), parameter `s` = segment index + local t.
  Exact {
    segments: Vec<RationalBezierCurve2>,
    /// False when the source is only within tolerance of the 3D edge (plane projection of
    /// an edge slightly off the plane).
    exact: bool,
  },
  /// Projection of `chain3`, parameter `s` = segment index + local t of the 3D chain.
  Projected,
  /// Straight native segment between the end samples (pole segments), `s ∈ [0, 1]`.
  Line,
}

impl NativeCurve {
  pub fn start(&self) -> Vec2<f64> {
    self.samples[0].uv
  }

  pub fn end(&self) -> Vec2<f64> {
    self.samples[self.samples.len() - 1].uv
  }

  pub fn set_start(&mut self, uv: Vec2<f64>) {
    self.samples[0].uv = uv;
  }

  pub fn set_end(&mut self, uv: Vec2<f64>) {
    let n = self.samples.len();
    self.samples[n - 1].uv = uv;
  }

  pub fn s_end(&self) -> f64 {
    self.samples[self.samples.len() - 1].s
  }

  /// 3D position of the curve start (the vertex for edge curves).
  pub fn start3(&self, fc: &FaceCtx) -> Vec3<f64> {
    match self.chain3.first() {
      Some(c) => c.start(),
      None => fc.surface.eval(self.start()),
    }
  }

  pub fn end3(&self, fc: &FaceCtx) -> Vec3<f64> {
    match self.chain3.last() {
      Some(c) => c.end(),
      None => fc.surface.eval(self.end()),
    }
  }

  pub fn translate(&mut self, d: Vec2<f64>) {
    if d.x == 0. && d.y == 0. {
      return;
    }
    self.shift += d;
    self.samples.iter_mut().for_each(|s| s.uv += d);
    if let NativeGeom::Exact { segments, .. } = &mut self.geom {
      segments.iter_mut().for_each(|c| *c = c.translate(d));
    }
  }

  pub fn reverse(&mut self) {
    let end = self.s_end();
    self.samples.reverse();
    self.samples.iter_mut().for_each(|s| s.s = end - s.s);
    reverse_chain(&mut self.chain3);
    if let NativeGeom::Exact { segments, .. } = &mut self.geom {
      reverse_chain(segments);
    }
    if let TrimCurveSource::Edge { reversed, .. } = &mut self.source {
      *reversed = !*reversed;
    }
  }

  /// Native position at parameter `s`: exact for exact curves, a projection seeded by the
  /// sample interpolation for projected curves.
  pub fn native_at(&self, fc: &FaceCtx, s: f64) -> Option<Vec2<f64>> {
    match &self.geom {
      NativeGeom::Exact { segments, .. } => Some(chain_eval(segments, s)),
      NativeGeom::Line => {
        let (a, b) = (self.start(), self.end());
        Some(a + (b - a) * s.clamp(0., 1.))
      }
      NativeGeom::Projected => {
        let guess = self.interpolate(s) - self.shift;
        let p = chain_eval(&self.chain3, s);
        let (uv, _) = project_point(fc, p, Some(guess)).ok()?;
        Some(uv + self.shift)
      }
    }
  }

  /// Linear interpolation of the samples at `s`.
  pub fn interpolate(&self, s: f64) -> Vec2<f64> {
    let samples = &self.samples;
    let i = samples.partition_point(|x| x.s <= s);
    if i == 0 {
      return samples[0].uv;
    }
    if i >= samples.len() {
      return samples[samples.len() - 1].uv;
    }
    let (a, b) = (samples[i - 1], samples[i]);
    let f = if b.s > a.s {
      (s - a.s) / (b.s - a.s)
    } else {
      0.
    };
    a.uv + (b.uv - a.uv) * f
  }
}

pub(super) fn reverse_chain<P: HomogeneousPoint>(chain: &mut [RationalBezierCurve<P>]) {
  chain.reverse();
  chain.iter_mut().for_each(|c| *c = c.reversed());
}

/// Chain point at `s` = segment index + local t (clamped to the chain).
pub(super) fn chain_eval<P: HomogeneousPoint>(
  chain: &[RationalBezierCurve<P>],
  s: f64,
) -> P::Euclid {
  let n = chain.len();
  let i = (s.floor().max(0.) as usize).min(n - 1);
  chain[i].eval((s - i as f64).clamp(0., 1.))
}

/// Build the native curves of one loop (in traversal order, before orientation handling).
/// Zero length edges are skipped.
pub(super) fn build_native_loop(
  fc: &FaceCtx,
  uses: &[OrientedEdgeUse],
  edges: &mut Edge3Cache,
  warnings: &mut Vec<String>,
) -> ConvertResult<Vec<NativeCurve>> {
  let mut curves: Vec<NativeCurve> = Vec::new();
  for edge_use in uses {
    let edge3 = edges.get(fc.model, edge_use.edge, fc.ctx)?.clone();
    if edge3.segments.is_empty() {
      continue;
    }
    let prev_end = curves.last().map(|c| c.end());
    let curve = build_curve(fc, edge_use, &edge3, prev_end, warnings)?;
    curves.push(curve);
  }
  Ok(curves)
}

fn build_curve(
  fc: &FaceCtx,
  edge_use: &OrientedEdgeUse,
  edge3: &Edge3,
  prev_end: Option<Vec2<f64>>,
  warnings: &mut Vec<String>,
) -> ConvertResult<NativeCurve> {
  let edge = fc
    .model
    .edges
    .get(edge_use.edge.index())
    .ok_or_else(|| ConvertError::invalid(None, "dangling edge id"))?;
  let reversed = !edge_use.orientation;
  let source = TrimCurveSource::Edge {
    edge: edge_use.edge,
    reversed,
  };
  let mut chain3 = edge3.segments.clone();
  if reversed {
    reverse_chain(&mut chain3);
  }

  let pcurves: Vec<_> = edge
    .pcurves
    .iter()
    .filter(|p| p.surface == fc.face.surface)
    .collect();
  if !pcurves.is_empty() {
    match pcurve_curve(fc, edge_use, edge3, &pcurves, prev_end) {
      Ok(Some((segments, iso))) => {
        return Ok(exact_curve(fc, source, chain3, segments, true, iso));
      }
      Ok(None) => {}
      Err(e) => warnings.push(format!(
        "edge #{}: pcurve not usable ({e}), projecting the 3D edge instead",
        edge.step_id
      )),
    }
  }

  if let Some(SurfaceGeom::Plane(frame)) = fc.geom {
    let (segments, off_plane) = plane_projection(frame, &chain3);
    if off_plane > GAP_FACTOR * fc.ctx.tol {
      return Err(ConvertError::invalid(
        Some(edge.step_id),
        format!(
          "edge is {off_plane:e} away from the face plane (tol {:e})",
          fc.ctx.tol
        ),
      ));
    }
    let exact = off_plane <= EXACT_FACTOR * fc.ctx.fit_tol;
    return Ok(exact_curve(fc, source, chain3, segments, exact, None));
  }

  let samples = project_samples(fc, &chain3, prev_end).map_err(|e| e.at(edge.step_id))?;
  let mut curve = NativeCurve {
    source,
    geom: NativeGeom::Projected,
    chain3,
    samples,
    shift: Vec2::new(0., 0.),
    iso: None,
    iso_exact: false,
    iso_dev: [0., 0.],
  };
  curve.iso_dev = iso_deviation(fc, &curve.samples);
  curve.iso = detected_iso(fc, &curve.iso_dev);
  let hint = fc.geom.and_then(|g| hint_iso(g, &edge3.hint, fc.ctx));
  curve.iso_exact = hint.is_some() && hint == curve.iso;
  Ok(curve)
}

/// Deviations below `EXACT_FACTOR · fit_tol` count as float noise.
pub(super) const EXACT_FACTOR: f64 = 1e-3;

fn detected_iso(fc: &FaceCtx, iso_dev: &[f64; 2]) -> Option<usize> {
  let limit = EXACT_FACTOR * fc.ctx.fit_tol;
  match (iso_dev[0] <= limit, iso_dev[1] <= limit) {
    (true, false) => Some(0),
    (false, true) => Some(1),
    _ => None,
  }
}

fn exact_curve(
  fc: &FaceCtx,
  source: TrimCurveSource,
  chain3: Vec<RationalBezierCurve3>,
  segments: Vec<RationalBezierCurve2>,
  exact: bool,
  iso: Option<IsoLine>,
) -> NativeCurve {
  let mut samples = Vec::with_capacity(segments.len() * EXACT_SAMPLES + 1);
  for (i, seg) in segments.iter().enumerate() {
    for k in 0..EXACT_SAMPLES {
      let t = k as f64 / EXACT_SAMPLES as f64;
      samples.push(Sample {
        s: i as f64 + t,
        uv: seg.eval(t),
        pole: false,
      });
    }
  }
  if let Some(last) = segments.last() {
    samples.push(Sample {
      s: segments.len() as f64,
      uv: last.end(),
      pole: false,
    });
  }
  let iso_dev = iso_deviation(fc, &samples);
  let iso = match iso {
    Some(IsoLine::U(_)) => Some(0),
    Some(IsoLine::V(_)) => Some(1),
    None => None,
  };
  NativeCurve {
    source,
    geom: NativeGeom::Exact { segments, exact },
    chain3,
    samples,
    shift: Vec2::new(0., 0.),
    iso: iso.or_else(|| detected_iso(fc, &iso_dev)),
    iso_exact: iso.is_some(),
    iso_dev,
  }
}

const EXACT_SAMPLES: usize = 16;

/// The pcurve (among the candidates on the face surface) whose start continues the previous
/// curve best, modulo periods. Seam edges have two candidates one period apart. `None` when
/// the pcurve is degenerate.
fn pcurve_curve(
  fc: &FaceCtx,
  edge_use: &OrientedEdgeUse,
  edge3: &Edge3,
  pcurves: &[&PcurveRef],
  prev_end: Option<Vec2<f64>>,
) -> ConvertResult<Option<(Vec<RationalBezierCurve2>, Option<IsoLine>)>> {
  let mut best: Option<(f64, Vec<RationalBezierCurve2>, Option<IsoLine>)> = None;
  let mut first_error = None;
  for pcurve in pcurves {
    let edge2 = match build_edge_pcurve(
      fc.model,
      edge_use.edge,
      pcurve.curve,
      fc.surface,
      edge3,
      fc.ctx,
    ) {
      Ok(e) => e,
      Err(e) => {
        first_error.get_or_insert(e);
        continue;
      }
    };
    if edge2.segments.is_empty() {
      continue;
    }
    let mut segments = edge2.segments;
    if !edge_use.orientation {
      reverse_chain(&mut segments);
    }
    let start = segments[0].start();
    let distance = prev_end.map_or(0., |p| periodic_distance(fc, start, p));
    if best.as_ref().is_none_or(|b| distance < b.0) {
      best = Some((distance, segments, edge2.iso));
    }
  }
  match (best, first_error) {
    (Some((_, segments, iso)), _) => Ok(Some((segments, iso))),
    (None, Some(e)) => Err(e),
    (None, None) => Ok(None),
  }
}

/// Native distance between `a` and the representative of `b` closest to it.
fn periodic_distance(fc: &FaceCtx, a: Vec2<f64>, b: Vec2<f64>) -> f64 {
  let mut d = a - b;
  for axis in 0..2 {
    if let Some(period) = fc.period[axis] {
      let x = coord(d, axis);
      set_coord(&mut d, axis, x - period * (x / period).round());
    }
  }
  d.length()
}

/// The exact native image of a 3D chain on a plane (an affine map of the homogeneous control
/// points), and the max distance of the control points from the plane.
fn plane_projection(
  frame: &Frame,
  chain3: &[RationalBezierCurve3],
) -> (Vec<RationalBezierCurve2>, f64) {
  let mut off_plane = 0f64;
  let segments = chain3
    .iter()
    .map(|c| {
      let control_points = c
        .control_points
        .iter()
        .map(|p| {
          let w = p.w;
          let d = Vec3::new(p.x, p.y, p.z) - frame.origin * w;
          off_plane = off_plane.max((d.dot(frame.z) / w).abs());
          Vec3::new(d.dot(frame.x), d.dot(frame.y), w)
        })
        .collect();
      RationalBezierCurve2::new(control_points)
    })
    .collect();
  (segments, off_plane)
}

/// Project a 3D point. At a pole the coordinate along the pole axis is set to the pole value
/// and the free coordinate is taken from the guess (to be fixed from the neighbor samples).
pub(super) fn project_point(
  fc: &FaceCtx,
  p: Vec3<f64>,
  guess: Option<Vec2<f64>>,
) -> ConvertResult<(Vec2<f64>, bool)> {
  let projection = fc
    .surface
    .project(p, guess)
    .ok_or_else(|| ConvertError::numerical(None, "surface projection failed"))?;
  if exceeds(projection.distance, GAP_FACTOR * fc.ctx.tol) {
    return Err(ConvertError::invalid(
      None,
      format!(
        "edge point ({}, {}, {}) is {:e} away from the surface (tol {:e})",
        p.x, p.y, p.z, projection.distance, fc.ctx.tol
      ),
    ));
  }
  let mut uv = projection.native;
  if let Some(pole) = sample_pole(fc, uv) {
    let (a, b) = (pole.axis.index(), pole.axis.other().index());
    set_coord(&mut uv, a, pole.value);
    if let Some(g) = guess {
      set_coord(&mut uv, b, coord(g, b));
    }
    return Ok((uv, true));
  }
  Ok((uv, false))
}

/// The pole a projected point lies on: its surface point is the pole position up to float
/// noise (the free coordinate is then meaningless).
fn sample_pole(fc: &FaceCtx, uv: Vec2<f64>) -> Option<Pole> {
  fc.pole_at(fc.surface.eval(uv), EXACT_FACTOR * fc.ctx.tol)
}

/// Initial steps per 3D segment (in segment parameter) of the adaptive sampling.
const INITIAL_STEPS: f64 = 8.;
/// Max halvings of the initial step.
const MAX_HALVINGS: i32 = 12;

/// Adaptive samples of the projection of `chain3`, each projection seeded with the previous
/// result. A step is accepted when the native midpoint of the step maps within `tol` of the
/// projected chain midpoint (so linear interpolation of the samples is a good guess and no
/// projection jumped to another branch).
fn project_samples(
  fc: &FaceCtx,
  chain3: &[RationalBezierCurve3],
  start_guess: Option<Vec2<f64>>,
) -> ConvertResult<Vec<Sample>> {
  let n = chain3.len() as f64;
  let h_max = 1. / INITIAL_STEPS;
  let h_min = h_max * 0.5f64.powi(MAX_HALVINGS);
  let table_tol = fc.ctx.tol;
  let (uv0, pole0) = project_point(fc, chain_eval(chain3, 0.), start_guess)?;
  let mut out = vec![Sample {
    s: 0.,
    uv: uv0,
    pole: pole0,
  }];
  let mut s = 0.;
  let mut h = h_max;
  while s < n {
    let last = out[out.len() - 1];
    let s1 = (s + h).min(n);
    let sm = 0.5 * (s + s1);
    let (uv1, pole1) = project_point(fc, chain_eval(chain3, s1), Some(last.uv))?;
    let (uvm, polem) = project_point(fc, chain_eval(chain3, sm), Some((last.uv + uv1) * 0.5))?;
    let error = if last.pole || pole1 || polem {
      0.
    } else {
      (fc.surface.eval((last.uv + uv1) * 0.5) - fc.surface.eval(uvm)).length()
    };
    if exceeds(error, table_tol) && h > h_min {
      h *= 0.5;
      continue;
    }
    if exceeds(error, 100. * table_tol) {
      return Err(ConvertError::numerical(
        None,
        format!("edge projection is not continuous near chain parameter {sm} (jump {error:e})"),
      ));
    }
    out.push(Sample {
      s: sm,
      uv: uvm,
      pole: polem,
    });
    out.push(Sample {
      s: s1,
      uv: uv1,
      pole: pole1,
    });
    s = s1;
    if error <= 0.25 * table_tol {
      h = (h * 2.).min(h_max);
    }
  }
  fix_pole_samples(fc, &mut out);
  Ok(out)
}

/// At a pole the free coordinate is arbitrary: take it from the nearest sample of the same
/// curve that is not at the pole (the direction the curve approaches the pole from).
fn fix_pole_samples(fc: &FaceCtx, samples: &mut [Sample]) {
  if !samples.iter().any(|s| s.pole) {
    return;
  }
  let n = samples.len();
  for i in 0..n {
    if !samples[i].pole {
      continue;
    }
    let Some(pole) = sample_pole(fc, samples[i].uv) else {
      continue;
    };
    let b = pole.axis.other().index();
    let before = (0..i).rev().find(|k| !samples[*k].pole);
    let after = (i + 1..n).find(|k| !samples[*k].pole);
    let nearest = match (before, after) {
      (Some(x), Some(y)) => Some(if i - x <= y - i { x } else { y }),
      (x, y) => x.or(y),
    };
    if let Some(k) = nearest {
      let value = coord(samples[k].uv, b);
      set_coord(&mut samples[i].uv, b, value);
    }
  }
}

/// Per native axis, the max 3D effect of the coordinate variation from the first sample:
/// `|∂S/∂a| · |a_k - a_0|` over the samples.
fn iso_deviation(fc: &FaceCtx, samples: &[Sample]) -> [f64; 2] {
  let first = samples[0].uv;
  let mut dev = [0f64; 2];
  for sample in samples {
    let (_, su, sv) = fc.surface.eval_with_partials(sample.uv);
    let d = sample.uv - first;
    dev[0] = dev[0].max(su.length() * d.x.abs());
    dev[1] = dev[1].max(sv.length() * d.y.abs());
  }
  dev.map(|d| if d.is_nan() { f64::INFINITY } else { d })
}
