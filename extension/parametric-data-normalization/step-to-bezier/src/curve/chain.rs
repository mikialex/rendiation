//! Dimension generic (2D / 3D) exact Bézier chains carrying their native parameterization.
//!
//! A chain is a list of pieces, piece `i` covering the native interval
//! `breaks[i]..breaks[i+1]`. A piece is either a Bézier curve whose parameter is an affine
//! function of the native parameter, or an arc of a conic whose angle is
//! `a0 + dir·(s - breaks[i])` (`dir = ±1`, so the native parameter has unit angular speed, which
//! is what `SegmentMap::CircularArc` assumes).

use std::f64::consts::{FRAC_PI_2, TAU};

use rendiation_algebra::*;

use crate::math::*;

/// Homogeneous point types usable in chains: `Vec3<f64>` for 2D curves, `Vec4<f64>` for 3D.
pub(super) trait CurvePoint: HomogeneousPoint<Euclid: PartialEq> {
  /// Exact rational quadratic of the conic arc over the increasing angle range `[a0, a1]`.
  fn arc(conic: &Conic<Self::Euclid>, a0: f64, a1: f64) -> RationalBezierCurve<Self>;
  fn is_finite(p: Self::Euclid) -> bool;
  /// Translate the Euclidean point by `d`, keeping the weight.
  fn translated(self, d: Self::Euclid) -> Self;
}

impl CurvePoint for Vec3<f64> {
  fn arc(c: &Conic<Vec2<f64>>, a0: f64, a1: f64) -> RationalBezierCurve2 {
    elliptic_arc_2d(c.center, c.x, c.y, c.rx, c.ry, a0, a1)
  }
  fn is_finite(p: Vec2<f64>) -> bool {
    p.x.is_finite() && p.y.is_finite()
  }
  fn translated(self, d: Vec2<f64>) -> Self {
    Vec3::new(self.x + d.x * self.z, self.y + d.y * self.z, self.z)
  }
}

impl CurvePoint for Vec4<f64> {
  fn arc(c: &Conic<Vec3<f64>>, a0: f64, a1: f64) -> RationalBezierCurve3 {
    elliptic_arc_3d(c.center, c.x, c.y, c.rx, c.ry, a0, a1)
  }
  fn is_finite(p: Vec3<f64>) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
  }
  fn translated(self, d: Vec3<f64>) -> Self {
    Vec4::new(
      self.x + d.x * self.w,
      self.y + d.y * self.w,
      self.z + d.z * self.w,
      self.w,
    )
  }
}

/// `center + rx·cos(a)·x + ry·sin(a)·y`, `x` and `y` orthonormal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Conic<E> {
  pub center: E,
  pub x: E,
  pub y: E,
  pub rx: f64,
  pub ry: f64,
}

pub(super) fn conic_point<P: CurvePoint>(c: &Conic<P::Euclid>, angle: f64) -> P::Euclid {
  c.center + c.x * (c.rx * angle.cos()) + c.y * (c.ry * angle.sin())
}

/// The analytic curve a chain was built from (kept through trimming), used for edge hints and
/// closed form projection.
#[derive(Debug, Clone, Copy)]
pub(super) enum Analytic<E> {
  Line { origin: E, vector: E },
  Conic(Conic<E>),
  Other,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PieceKind<E> {
  Affine,
  Arc { conic: Conic<E>, a0: f64, dir: f64 },
}

#[derive(Debug, Clone)]
pub(super) struct Piece<P: CurvePoint> {
  pub curve: RationalBezierCurve<P>,
  pub kind: PieceKind<P::Euclid>,
}

impl<P: CurvePoint> Piece<P> {
  pub fn affine(curve: RationalBezierCurve<P>) -> Self {
    Self {
      curve,
      kind: PieceKind::Affine,
    }
  }

  /// Arc starting at angle `a0`, running `span` radians in direction `dir`.
  pub fn arc(conic: Conic<P::Euclid>, a0: f64, dir: f64, span: f64) -> Self {
    let curve = if dir > 0. {
      P::arc(&conic, a0, a0 + span)
    } else {
      // The arc parameterization is symmetric (`t(span - θ) = 1 - t(θ)`), so the reversed
      // curve keeps the `CircularArc` map in the decreasing angle direction.
      P::arc(&conic, a0 - span, a0).reversed()
    };
    Self {
      curve,
      kind: PieceKind::Arc { conic, a0, dir },
    }
  }

  fn translated(&self, d: P::Euclid) -> Self {
    let curve = RationalBezierCurve::new(
      self
        .curve
        .control_points
        .iter()
        .map(|p| p.translated(d))
        .collect(),
    );
    let kind = match self.kind {
      PieceKind::Affine => PieceKind::Affine,
      PieceKind::Arc { conic, a0, dir } => PieceKind::Arc {
        conic: Conic {
          center: conic.center + d,
          ..conic
        },
        a0,
        dir,
      },
    };
    Self { curve, kind }
  }
}

/// Periodic extension: `C(s + length) = C(s) + shift`. `shift` is `None` for curves that are
/// truly closed; pcurves may be closed only modulo a surface period, then `shift` is that
/// period vector in uv.
#[derive(Debug, Clone, Copy)]
pub(super) struct Period<E> {
  pub length: f64,
  pub shift: Option<E>,
}

#[derive(Debug, Clone)]
pub(super) struct Chain<P: CurvePoint> {
  /// Strictly increasing, `pieces.len() + 1` values.
  pub breaks: Vec<f64>,
  pub pieces: Vec<Piece<P>>,
  /// When set, `length == breaks[n] - breaks[0]`.
  pub period: Option<Period<P::Euclid>>,
  pub analytic: Analytic<P::Euclid>,
}

/// Pieces shorter than this fraction of their native span are merged into a neighbour when
/// restricting (a vertex parameter that coincides with a knot up to rounding).
const SLIVER: f64 = 1e-9;

impl<P: CurvePoint> Chain<P> {
  pub fn new(breaks: Vec<f64>, pieces: Vec<Piece<P>>, analytic: Analytic<P::Euclid>) -> Self {
    debug_assert_eq!(breaks.len(), pieces.len() + 1);
    Self {
      breaks,
      pieces,
      period: None,
      analytic,
    }
  }

  /// The full conic as arcs of at most π/2, periodic with period 2π.
  pub fn conic(conic: Conic<P::Euclid>) -> Self {
    let mut chain = Self::conic_range(conic, 0., TAU);
    chain.period = Some(Period {
      length: TAU,
      shift: None,
    });
    chain
  }

  /// Arcs of the conic over the increasing angle range `[a0, a1]`, native parameter = angle.
  pub fn conic_range(conic: Conic<P::Euclid>, a0: f64, a1: f64) -> Self {
    let ranges = split_angle_range(a0, a1, FRAC_PI_2);
    let mut breaks = vec![a0];
    let mut pieces = Vec::with_capacity(ranges.len());
    for (lo, hi) in ranges {
      pieces.push(Piece::arc(conic, lo, 1., hi - lo));
      breaks.push(hi);
    }
    Self::new(breaks, pieces, Analytic::Conic(conic))
  }

  pub fn domain(&self) -> (f64, f64) {
    (self.breaks[0], self.breaks[self.pieces.len()])
  }

  pub fn length(&self) -> f64 {
    let (a, b) = self.domain();
    b - a
  }

  pub fn start_point(&self) -> P::Euclid {
    self.pieces[0].curve.start()
  }

  pub fn end_point(&self) -> P::Euclid {
    self.pieces[self.pieces.len() - 1].curve.end()
  }

  pub fn curves(&self) -> Vec<RationalBezierCurve<P>> {
    self.pieces.iter().map(|p| p.curve.clone()).collect()
  }

  /// Largest distance from the first control point to any control point, a scale for
  /// relative tolerances.
  pub fn extent(&self) -> f64 {
    let origin = self.start_point();
    self
      .pieces
      .iter()
      .flat_map(|p| &p.curve.control_points)
      .map(|p| P::euclid_distance(p.to_euclid(), origin))
      .fold(0., f64::max)
  }

  /// Make the chain periodic when its end points coincide within `tol`.
  pub fn detect_closed(&mut self, tol: f64) {
    if self.period.is_none() && P::euclid_distance(self.start_point(), self.end_point()) <= tol {
      self.period = Some(Period {
        length: self.length(),
        shift: None,
      });
    }
  }

  /// Period index `k`, piece index and native value reduced into the base period. Without a
  /// period, out of domain values use the first / last piece (extrapolation).
  fn locate(&self, s: f64) -> (i64, usize, f64) {
    let n = self.pieces.len();
    let (k, local) = match self.period {
      Some(period) => {
        let k = ((s - self.breaks[0]) / period.length).floor();
        if !k.is_finite() {
          return (0, 0, s);
        }
        (k as i64, s - k * period.length)
      }
      None => (0, s),
    };
    let i = self
      .breaks
      .partition_point(|b| *b <= local)
      .saturating_sub(1)
      .min(n - 1);
    (k, i, local)
  }

  fn shift_for(&self, k: i64) -> Option<P::Euclid> {
    match self.period {
      Some(Period {
        shift: Some(shift), ..
      }) if k != 0 => Some(shift * k as f64),
      _ => None,
    }
  }

  pub fn eval(&self, s: f64) -> P::Euclid {
    let (k, i, local) = self.locate(s);
    let p = self.eval_piece(i, local);
    match self.shift_for(k) {
      Some(d) => p + d,
      None => p,
    }
  }

  fn eval_piece(&self, i: usize, s: f64) -> P::Euclid {
    let (lo, hi) = (self.breaks[i], self.breaks[i + 1]);
    match self.pieces[i].kind {
      PieceKind::Affine => self.pieces[i].curve.eval((s - lo) / (hi - lo)),
      PieceKind::Arc { conic, a0, dir } => conic_point::<P>(&conic, a0 + dir * (s - lo)),
    }
  }

  /// Native parameter of local Bézier parameter `t` of piece `i`.
  pub fn piece_param(&self, i: usize, t: f64) -> f64 {
    let (lo, hi) = (self.breaks[i], self.breaks[i + 1]);
    if t <= 0. {
      return lo;
    }
    if t >= 1. {
      return hi;
    }
    match self.pieces[i].kind {
      PieceKind::Affine => lo + t * (hi - lo),
      PieceKind::Arc { .. } => lo + arc_angle_from_t(t, hi - lo),
    }
  }

  /// Native parameter of the closest point (in the base period for periodic chains) and the
  /// distance. Circles use the closed form, everything else the exact Bézier pieces.
  pub fn project(&self, p: P::Euclid) -> (f64, f64) {
    if let Some(s) = self.project_circle(p) {
      return (s, P::euclid_distance(self.eval(s), p));
    }
    let curves = self.curves();
    match closest_point_on_chain(&curves, p) {
      Some((i, t, d)) => (self.piece_param(i, t), d),
      None => (self.breaks[0], P::euclid_distance(self.start_point(), p)),
    }
  }

  fn project_circle(&self, p: P::Euclid) -> Option<f64> {
    let Analytic::Conic(conic) = self.analytic else {
      return None;
    };
    if conic.rx != conic.ry {
      return None;
    }
    let d = p - conic.center;
    let (cx, cy) = (P::euclid_dot(d, conic.x), P::euclid_dot(d, conic.y));
    if cx == 0. && cy == 0. {
      return None;
    }
    let angle = cy.atan2(cx);
    let tau = TAU;
    for (i, piece) in self.pieces.iter().enumerate() {
      let PieceKind::Arc { conic: c, a0, dir } = piece.kind else {
        return None;
      };
      if c != conic {
        return None;
      }
      let span = self.breaks[i + 1] - self.breaks[i];
      let local = (dir * (angle - a0)).rem_euclid(tau);
      if local <= span {
        return Some(self.breaks[i] + local);
      }
      // Just before the piece start (rounding of the angle).
      if tau - local <= 1e-12 {
        return Some(self.breaks[i]);
      }
    }
    // Outside a trimmed arc: the closest point is an end point.
    None
  }

  /// The part over the native interval `[a, b]`, `a < b`. Periodic chains accept any `a` and
  /// spans up to one period (longer spans are cut to one period); pieces of later periods are
  /// translated by the period shift. Non periodic chains clamp the interval to the domain. The
  /// result has exact breaks `a` and `b` and is periodic when it spans exactly one period.
  pub fn restrict(&self, a: f64, b: f64) -> Chain<P> {
    let n = self.pieces.len();
    // (piece, period index, clipped native interval in the base period frame)
    let mut parts: Vec<(usize, i64, f64, f64)> = Vec::new();
    let (a, b, offset_of) = match self.period {
      Some(period) => (a, b.min(a + period.length), period.length),
      None => {
        let (d0, d1) = self.domain();
        (a.max(d0), b.min(d1), 0.)
      }
    };
    let k0 = match self.period {
      Some(period) => ((a - self.breaks[0]) / period.length).floor(),
      None => 0.,
    };
    if !(k0.is_finite() && a < b) {
      return self.degenerate(a);
    }
    let rounds = if self.period.is_some() { 2 * n + 2 } else { n };
    for j in 0..rounds {
      let k = k0 as i64 + (j / n) as i64;
      let i = j % n;
      let off = k as f64 * offset_of;
      let (lo, hi) = (self.breaks[i] + off, self.breaks[i + 1] + off);
      if hi <= a {
        continue;
      }
      if lo >= b {
        break;
      }
      parts.push((i, k, lo.max(a), hi.min(b)));
    }
    if parts.is_empty() {
      return self.degenerate(a);
    }
    // Exact outer breaks.
    parts[0].2 = a;
    let last = parts.len() - 1;
    parts[last].3 = b;
    merge_slivers(&mut parts, |i| self.breaks[i + 1] - self.breaks[i]);

    let mut breaks = vec![a];
    let mut pieces = Vec::with_capacity(parts.len());
    for &(i, k, c0, c1) in &parts {
      let off = k as f64 * offset_of;
      pieces.push(self.clip_piece(i, k, c0 - off, c1 - off));
      breaks.push(c1);
    }
    let mut out = Chain::new(breaks, pieces, self.analytic);
    out.merge_arcs();
    if let Some(period) = self.period
      && (b - a) >= period.length * (1. - 1e-12)
    {
      out.period = Some(Period {
        length: b - a,
        shift: period.shift,
      });
    }
    out
  }

  fn degenerate(&self, a: f64) -> Chain<P> {
    let p = P::from_euclid(self.eval(a), 1.);
    Chain::new(
      vec![a, a],
      vec![Piece::affine(RationalBezierCurve::new(vec![p, p]))],
      self.analytic,
    )
  }

  /// Piece `i` restricted to `[c0, c1]` (base period frame, may extend slightly beyond the
  /// piece after sliver merging), translated for period index `k`.
  fn clip_piece(&self, i: usize, k: i64, c0: f64, c1: f64) -> Piece<P> {
    let (lo, hi) = (self.breaks[i], self.breaks[i + 1]);
    let piece = &self.pieces[i];
    let clipped = match piece.kind {
      PieceKind::Affine => {
        let t0 = if c0 == lo { 0. } else { (c0 - lo) / (hi - lo) };
        let t1 = if c1 == hi { 1. } else { (c1 - lo) / (hi - lo) };
        Piece::affine(piece.curve.sub_curve(t0, t1))
      }
      PieceKind::Arc { conic, a0, dir } => {
        if c0 == lo && c1 == hi {
          piece.clone()
        } else {
          Piece::arc(conic, a0 + dir * (c0 - lo), dir, c1 - c0)
        }
      }
    };
    match self.shift_for(k) {
      Some(d) => clipped.translated(d),
      None => clipped,
    }
  }

  /// Re-split runs of consecutive arcs of the same conic into equal arcs of at most π/2, so
  /// trimming never leaves tiny arcs at the ends.
  pub fn merge_arcs(&mut self) {
    let n = self.pieces.len();
    let mut breaks = vec![self.breaks[0]];
    let mut pieces = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
      let PieceKind::Arc { conic, a0, dir } = self.pieces[i].kind else {
        pieces.push(self.pieces[i].clone());
        breaks.push(self.breaks[i + 1]);
        i += 1;
        continue;
      };
      let mut j = i + 1;
      while j < n {
        let PieceKind::Arc {
          conic: c,
          a0: next_a0,
          dir: d,
        } = self.pieces[j].kind
        else {
          break;
        };
        // Angles of later periods restart at the base value, compare modulo 2π.
        let gap = (next_a0 - a0 - dir * (self.breaks[j] - self.breaks[i])).rem_euclid(TAU);
        if c != conic || d != dir || gap.min(TAU - gap) > 1e-9 {
          break;
        }
        j += 1;
      }
      let (s0, s1) = (self.breaks[i], self.breaks[j]);
      if j == i + 1 && s1 - s0 <= FRAC_PI_2 {
        pieces.push(self.pieces[i].clone());
        breaks.push(s1);
      } else {
        for (lo, hi) in split_angle_range(s0, s1, FRAC_PI_2) {
          pieces.push(Piece::arc(conic, a0 + dir * (lo - s0), dir, hi - lo));
          breaks.push(hi);
        }
      }
      i = j;
    }
    self.breaks = breaks;
    self.pieces = pieces;
  }

  pub fn shifted(mut self, d: f64) -> Self {
    for b in &mut self.breaks {
      *b += d;
    }
    self
  }

  /// Reparameterize by `s' = c - s` (reverses the direction).
  pub fn reversed_param(&self, c: f64) -> Self {
    let n = self.pieces.len();
    let breaks = self.breaks.iter().rev().map(|b| c - b).collect();
    let pieces = (0..n)
      .rev()
      .map(|i| {
        let piece = &self.pieces[i];
        let curve = piece.curve.reversed();
        let kind = match piece.kind {
          PieceKind::Affine => PieceKind::Affine,
          PieceKind::Arc { conic, a0, dir } => PieceKind::Arc {
            conic,
            a0: a0 + dir * (self.breaks[i + 1] - self.breaks[i]),
            dir: -dir,
          },
        };
        Piece { curve, kind }
      })
      .collect();
    Self {
      breaks,
      pieces,
      period: self.period.map(|p| Period {
        length: p.length,
        shift: p.shift.map(|s| s * -1.),
      }),
      analytic: self.analytic,
    }
  }

  /// Bézier segments and the native parameter map. The map is periodic only for truly closed
  /// chains (no period shift).
  pub fn to_param_space(&self) -> (PiecewiseParamMap, Vec<RationalBezierCurve<P>>) {
    let segments = self
      .pieces
      .iter()
      .map(|p| match p.kind {
        PieceKind::Affine => SegmentMap::Affine,
        PieceKind::Arc { .. } => SegmentMap::CircularArc,
      })
      .collect();
    let period = match self.period {
      Some(Period { shift: None, .. }) => Some(self.length()),
      _ => None,
    };
    (
      PiecewiseParamMap::new(self.breaks.clone(), segments, period),
      self.curves(),
    )
  }
}

/// Merge parts much shorter than their piece span into a neighbour (the neighbour then extends
/// by the sliver, through extrapolation of its own piece). Keeps at least one part.
fn merge_slivers(parts: &mut Vec<(usize, i64, f64, f64)>, span: impl Fn(usize) -> f64) {
  let mut i = 0;
  while i < parts.len() && parts.len() > 1 {
    let (piece, _, c0, c1) = parts[i];
    if c1 - c0 > SLIVER * span(piece) {
      i += 1;
      continue;
    }
    if i + 1 < parts.len() {
      parts[i + 1].2 = c0;
    } else {
      parts[i - 1].3 = c1;
    }
    parts.remove(i);
  }
}

/// A resolved curve: an unbounded line, or a chain.
#[derive(Debug, Clone)]
pub(super) enum Resolved<P: CurvePoint> {
  Line {
    origin: P::Euclid,
    vector: P::Euclid,
  },
  Chain(Chain<P>),
}

impl<P: CurvePoint> Resolved<P> {
  pub fn eval(&self, s: f64) -> P::Euclid {
    match self {
      Resolved::Line { origin, vector } => *origin + *vector * s,
      Resolved::Chain(chain) => chain.eval(s),
    }
  }

  pub fn project(&self, p: P::Euclid) -> (f64, f64) {
    match self {
      Resolved::Line { origin, vector } => {
        let s = P::euclid_dot(p - *origin, *vector) / P::euclid_dot(*vector, *vector);
        (s, P::euclid_distance(self.eval(s), p))
      }
      Resolved::Chain(chain) => chain.project(p),
    }
  }

  pub fn period(&self) -> Option<Period<P::Euclid>> {
    match self {
      Resolved::Line { .. } => None,
      Resolved::Chain(chain) => chain.period,
    }
  }

  /// See `Chain::restrict`. A line becomes one exact linear segment.
  pub fn restrict(&self, a: f64, b: f64) -> Chain<P> {
    match self {
      Resolved::Line { origin, vector } => Chain::new(
        vec![a, b],
        vec![Piece::affine(RationalBezierCurve::line(
          *origin + *vector * a,
          *origin + *vector * b,
        ))],
        Analytic::Line {
          origin: *origin,
          vector: *vector,
        },
      ),
      Resolved::Chain(chain) => chain.restrict(a, b),
    }
  }
}
