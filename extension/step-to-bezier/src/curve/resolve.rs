//! `CurveGeom` / `Curve2Geom` to `Resolved` chains, with the native parameterizations of
//! ISO 10303-42.
//!
//! Parameterizations implemented:
//! - `LINE`: `origin + t·vector`, unbounded.
//! - `CIRCLE` / `ELLIPSE`: angle, periodic 2π, arcs of π/2 starting at angle 0.
//! - `B_SPLINE_CURVE`: knot parameter, one affine piece per non empty knot span.
//! - `POLYLINE`: parameter `i` at point `i`.
//! - `TRIMMED_CURVE`: `s = t - t1` when `sense_agreement` is true and `s = t1 - t` otherwise
//!   (`t` basis parameter, `t1` / `t2` the trim parameters), so `s` runs from 0 at trim 1 to
//!   `|t2 - t1|` at trim 2 and the curve runs in its own direction. On periodic bases the
//!   interval wraps through the period in the sense direction; on open bases the trims must be
//!   ordered consistently with `sense_agreement`.
//! - `COMPOSITE_CURVE`: segment `k` covers `[S_k, S_k + L_k]` with `S_k` the sum of the
//!   parametric lengths `L_j` of the previous segments; inside a segment `s = S_k + (t - t0)` for
//!   `same_sense` true and `s = S_k + (t1 - t)` otherwise (`[t0, t1]` the parent domain).
//!
//! Bounded chains whose end points coincide within the closure tolerance become periodic with
//! the domain length as period (closed B-splines, closed composite curves, trimmed full
//! circles). The STEP `closed_curve` flag is not trusted.

use rendiation_algebra::*;

use super::chain::*;
use crate::brep::*;
use crate::math::*;
use crate::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum Trim<E> {
  Parameter(f64),
  Point(E),
}

/// Dimension independent view of one curve entity.
pub(super) enum Geom<'a, E> {
  Line {
    origin: E,
    vector: E,
  },
  Conic(Conic<E>),
  BSpline(&'a BSplineCurveData<E>),
  Polyline(&'a [E]),
  Trimmed {
    basis: usize,
    trim1: Vec<Trim<E>>,
    trim2: Vec<Trim<E>>,
    sense_agreement: bool,
  },
  Composite(Vec<(usize, bool)>),
  Unsupported(ConvertError),
}

pub(super) trait GeomSource {
  type P: CurvePoint;
  fn geom(&self, index: usize) -> ConvertResult<Geom<'_, <Self::P as HomogeneousPoint>::Euclid>>;
}

pub(super) struct Curves3<'a>(pub &'a BrepModel);
pub(super) struct Curves2<'a>(pub &'a BrepModel);

impl GeomSource for Curves3<'_> {
  type P = Vec4<f64>;
  fn geom(&self, index: usize) -> ConvertResult<Geom<'_, Vec3<f64>>> {
    let curve = self
      .0
      .curves
      .get(index)
      .ok_or_else(|| ConvertError::invalid(None, format!("curve id {index} out of range")))?;
    Ok(match curve {
      CurveGeom::Line { origin, vector } => Geom::Line {
        origin: *origin,
        vector: *vector,
      },
      CurveGeom::Circle { frame, radius } => Geom::Conic(conic3(frame, *radius, *radius)),
      CurveGeom::Ellipse {
        frame,
        semi_axis1,
        semi_axis2,
      } => Geom::Conic(conic3(frame, *semi_axis1, *semi_axis2)),
      CurveGeom::BSpline(data) => Geom::BSpline(data),
      CurveGeom::Polyline(points) => Geom::Polyline(points),
      CurveGeom::Trimmed {
        basis,
        trim1,
        trim2,
        sense_agreement,
      } => {
        let map = |t: &[TrimSelect]| {
          t.iter()
            .map(|t| match t {
              TrimSelect::Parameter(v) => Trim::Parameter(*v),
              TrimSelect::Point(p) => Trim::Point(*p),
            })
            .collect()
        };
        Geom::Trimmed {
          basis: basis.index(),
          trim1: map(trim1),
          trim2: map(trim2),
          sense_agreement: *sense_agreement,
        }
      }
      CurveGeom::Composite(segments) => {
        Geom::Composite(segments.iter().map(|(c, s)| (c.index(), *s)).collect())
      }
      CurveGeom::Offset { .. } => Geom::Unsupported(ConvertError::unsupported(
        None,
        "OFFSET_CURVE_3D is not supported",
      )),
      CurveGeom::Unsupported { step_id, type_name } => Geom::Unsupported(
        ConvertError::unsupported(Some(*step_id), format!("unsupported curve {type_name}")),
      ),
    })
  }
}

impl GeomSource for Curves2<'_> {
  type P = Vec3<f64>;
  fn geom(&self, index: usize) -> ConvertResult<Geom<'_, Vec2<f64>>> {
    let curve = self
      .0
      .curves_2d
      .get(index)
      .ok_or_else(|| ConvertError::invalid(None, format!("2D curve id {index} out of range")))?;
    Ok(match curve {
      Curve2Geom::Line { origin, vector } => Geom::Line {
        origin: *origin,
        vector: *vector,
      },
      Curve2Geom::Circle { frame, radius } => Geom::Conic(conic2(frame, *radius, *radius)),
      Curve2Geom::Ellipse {
        frame,
        semi_axis1,
        semi_axis2,
      } => Geom::Conic(conic2(frame, *semi_axis1, *semi_axis2)),
      Curve2Geom::BSpline(data) => Geom::BSpline(data),
      Curve2Geom::Polyline(points) => Geom::Polyline(points),
      Curve2Geom::Trimmed {
        basis,
        trim1,
        trim2,
        sense_agreement,
      } => {
        let map = |t: &[TrimSelect2]| {
          t.iter()
            .map(|t| match t {
              TrimSelect2::Parameter(v) => Trim::Parameter(*v),
              TrimSelect2::Point(p) => Trim::Point(*p),
            })
            .collect()
        };
        Geom::Trimmed {
          basis: basis.index(),
          trim1: map(trim1),
          trim2: map(trim2),
          sense_agreement: *sense_agreement,
        }
      }
      Curve2Geom::Composite(segments) => {
        Geom::Composite(segments.iter().map(|(c, s)| (c.index(), *s)).collect())
      }
      Curve2Geom::Unsupported { step_id, type_name } => Geom::Unsupported(
        ConvertError::unsupported(Some(*step_id), format!("unsupported 2D curve {type_name}")),
      ),
    })
  }
}

fn conic3(frame: &Frame, rx: f64, ry: f64) -> Conic<Vec3<f64>> {
  Conic {
    center: frame.origin,
    x: frame.x,
    y: frame.y,
    rx,
    ry,
  }
}

fn conic2(frame: &Frame2, rx: f64, ry: f64) -> Conic<Vec2<f64>> {
  Conic {
    center: frame.origin,
    x: frame.x,
    y: frame.y,
    rx,
    ry,
  }
}

/// Nesting limit for trimmed / composite curves (guards reference cycles in corrupt data).
const MAX_DEPTH: usize = 32;

/// Resolve curve `index`. `close_tol` is the distance under which chain end points are the same
/// point (closed curve detection); `None` uses `1e-9` times the curve extent.
pub(super) fn resolve<S: GeomSource>(
  src: &S,
  index: usize,
  close_tol: Option<f64>,
) -> ConvertResult<Resolved<S::P>> {
  resolve_at(src, index, close_tol, 0)
}

fn resolve_at<S: GeomSource>(
  src: &S,
  index: usize,
  close_tol: Option<f64>,
  depth: usize,
) -> ConvertResult<Resolved<S::P>> {
  if depth > MAX_DEPTH {
    return Err(ConvertError::invalid(
      None,
      "curve nesting too deep (reference cycle?)",
    ));
  }
  let mut chain = match src.geom(index)? {
    Geom::Line { origin, vector } => {
      if !(S::P::is_finite(origin) && S::P::is_finite(vector)) {
        return Err(ConvertError::invalid(None, "LINE with non finite data"));
      }
      if S::P::euclid_dot(vector, vector) <= 0. {
        return Err(ConvertError::invalid(None, "LINE with zero vector"));
      }
      return Ok(Resolved::Line { origin, vector });
    }
    Geom::Conic(conic) => {
      let finite = [conic.center, conic.x, conic.y]
        .into_iter()
        .all(S::P::is_finite);
      if !finite {
        return Err(ConvertError::invalid(
          None,
          "conic with non finite placement",
        ));
      }
      for r in [conic.rx, conic.ry] {
        if !(r.is_finite() && r > 0.) {
          return Err(ConvertError::invalid(
            None,
            format!("conic radius {r} is not positive"),
          ));
        }
      }
      Chain::conic(conic)
    }
    Geom::BSpline(data) => bspline_chain::<S::P>(data)?,
    Geom::Polyline(points) => polyline_chain::<S::P>(points)?,
    Geom::Trimmed {
      basis,
      trim1,
      trim2,
      sense_agreement,
    } => {
      let basis = resolve_at(src, basis, close_tol, depth + 1)?;
      trimmed_chain(&basis, &trim1, &trim2, sense_agreement)?
    }
    Geom::Composite(segments) => composite_chain(src, &segments, close_tol, depth)?,
    Geom::Unsupported(e) => return Err(e),
  };
  let tol = close_tol.unwrap_or_else(|| 1e-9 * chain.extent());
  chain.detect_closed(tol);
  Ok(Resolved::Chain(chain))
}

fn bspline_chain<P: CurvePoint>(data: &BSplineCurveData<P::Euclid>) -> ConvertResult<Chain<P>> {
  if data.degree == 0 {
    return Err(ConvertError::unsupported(
      None,
      "B-spline curve of degree 0",
    ));
  }
  if !data.control_points.iter().all(|p| P::is_finite(*p)) {
    return Err(ConvertError::invalid(
      None,
      "B-spline curve with non finite control points",
    ));
  }
  let control_points = match &data.weights {
    Some(w) => {
      if w.len() != data.control_points.len() {
        return Err(ConvertError::invalid(
          None,
          format!(
            "B-spline curve has {} weights for {} control points",
            w.len(),
            data.control_points.len()
          ),
        ));
      }
      data
        .control_points
        .iter()
        .zip(w)
        .map(|(p, w)| P::from_euclid(*p, *w))
        .collect()
    }
    None => data
      .control_points
      .iter()
      .map(|p| P::from_euclid(*p, 1.))
      .collect(),
  };
  let nurbs = NurbsCurve::new(data.degree, data.knots.clone(), control_points)?;
  let segments = nurbs.to_bezier_segments();
  let Some(((first, _), _)) = segments.first() else {
    return Err(ConvertError::invalid(
      None,
      "B-spline curve with empty domain",
    ));
  };
  let mut breaks = vec![*first];
  let mut pieces = Vec::with_capacity(segments.len());
  for ((_, hi), curve) in segments {
    breaks.push(hi);
    pieces.push(Piece::affine(curve));
  }
  Ok(Chain::new(breaks, pieces, Analytic::Other))
}

fn polyline_chain<P: CurvePoint>(points: &[P::Euclid]) -> ConvertResult<Chain<P>> {
  if points.len() < 2 {
    return Err(ConvertError::invalid(
      None,
      "POLYLINE with less than 2 points",
    ));
  }
  if !points.iter().all(|p| P::is_finite(*p)) {
    return Err(ConvertError::invalid(
      None,
      "POLYLINE with non finite points",
    ));
  }
  let breaks = (0..points.len()).map(|i| i as f64).collect();
  let pieces = points
    .windows(2)
    .map(|w| Piece::affine(RationalBezierCurve::line(w[0], w[1])))
    .collect();
  Ok(Chain::new(breaks, pieces, Analytic::Other))
}

fn trim_param<P: CurvePoint>(basis: &Resolved<P>, trims: &[Trim<P::Euclid>]) -> ConvertResult<f64> {
  let parameter = trims.iter().find_map(|t| match t {
    Trim::Parameter(v) => Some(*v),
    Trim::Point(_) => None,
  });
  if let Some(v) = parameter {
    if !v.is_finite() {
      return Err(ConvertError::invalid(None, "non finite trim parameter"));
    }
    return Ok(v);
  }
  let point = trims.iter().find_map(|t| match t {
    Trim::Point(p) => Some(*p),
    Trim::Parameter(_) => None,
  });
  match point {
    Some(p) if P::is_finite(p) => Ok(basis.project(p).0),
    Some(_) => Err(ConvertError::invalid(None, "non finite trim point")),
    None => Err(ConvertError::invalid(
      None,
      "TRIMMED_CURVE without trim value",
    )),
  }
}

fn trimmed_chain<P: CurvePoint>(
  basis: &Resolved<P>,
  trim1: &[Trim<P::Euclid>],
  trim2: &[Trim<P::Euclid>],
  sense_agreement: bool,
) -> ConvertResult<Chain<P>> {
  let t1 = trim_param(basis, trim1)?;
  let t2 = trim_param(basis, trim2)?;
  let (lo, hi) = match basis.period() {
    Some(period) => {
      let length = period.length;
      // Distance travelled from t1 in the sense direction; equal trims mean the full period.
      let mut d = if sense_agreement { t2 - t1 } else { t1 - t2 }.rem_euclid(length);
      if d <= 1e-12 * length || d >= length * (1. - 1e-12) {
        d = length;
      }
      if sense_agreement {
        (t1, t1 + d)
      } else {
        (t1 - d, t1)
      }
    }
    None => {
      let (lo, hi) = if sense_agreement { (t1, t2) } else { (t2, t1) };
      if lo >= hi {
        return Err(ConvertError::invalid(
          None,
          format!(
            "TRIMMED_CURVE trims {t1} -> {t2} contradict sense_agreement {sense_agreement} on an \
             open basis curve"
          ),
        ));
      }
      if let Resolved::Chain(chain) = basis {
        let (d0, d1) = chain.domain();
        let eps = 1e-9 * (d1 - d0);
        if lo < d0 - eps || hi > d1 + eps {
          return Err(ConvertError::invalid(
            None,
            format!("TRIMMED_CURVE range [{lo}, {hi}] outside basis domain [{d0}, {d1}]"),
          ));
        }
      }
      (lo, hi)
    }
  };
  let restricted = basis.restrict(lo, hi);
  Ok(if sense_agreement {
    restricted.shifted(-t1)
  } else {
    restricted.reversed_param(t1)
  })
}

fn composite_chain<S: GeomSource>(
  src: &S,
  segments: &[(usize, bool)],
  close_tol: Option<f64>,
  depth: usize,
) -> ConvertResult<Chain<S::P>> {
  if segments.is_empty() {
    return Err(ConvertError::invalid(
      None,
      "COMPOSITE_CURVE without segments",
    ));
  }
  let mut breaks = vec![0.];
  let mut pieces = Vec::new();
  let mut analytic = Analytic::Other;
  for (curve, same_sense) in segments {
    let Resolved::Chain(chain) = resolve_at(src, *curve, close_tol, depth + 1)? else {
      return Err(ConvertError::invalid(
        None,
        "unbounded LINE used as COMPOSITE_CURVE segment",
      ));
    };
    let (d0, d1) = chain.domain();
    let acc = breaks[breaks.len() - 1];
    let oriented = if *same_sense {
      chain.shifted(acc - d0)
    } else {
      chain.reversed_param(d1).shifted(acc)
    };
    breaks.extend_from_slice(&oriented.breaks[1..]);
    pieces.extend(oriented.pieces);
    if segments.len() == 1 {
      analytic = oriented.analytic;
    }
  }
  let mut chain = Chain::new(breaks, pieces, analytic);
  chain.merge_arcs();
  Ok(chain)
}
