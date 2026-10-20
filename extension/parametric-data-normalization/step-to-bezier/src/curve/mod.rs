//! STEP curve geometry to exact rational Bézier chains.
//!
//! Native parameterizations and the trimmed / composite conventions are documented in
//! `resolve.rs`. Edge conventions:
//! - `EDGE_CURVE.same_sense` true: the curve parameter increases from the start vertex to the
//!   end vertex, false: it decreases. The edge chain always runs from start to end vertex.
//! - Periodic curves (circles, ellipses, chains whose end points coincide within `ctx.tol`):
//!   the interval goes from the start vertex parameter in the `same_sense` direction to the end
//!   vertex parameter, wrapping through the period. Edges whose vertices coincide (same vertex,
//!   or distinct vertices within `ctx.tol`) cover the full period.
//! - Lines: the segment between the two vertices.
//! - Zero length edges (a loop edge on an open curve, a line edge with coincident vertices)
//!   yield an `Edge3` without segments.

use fast_hash_collection::FastHashMap;
use rendiation_algebra::*;

mod chain;
mod edge3;
mod pcurve;
#[cfg(test)]
mod real_files;
mod resolve;
#[cfg(test)]
mod tests;

use chain::*;
use resolve::*;

use crate::brep::*;
use crate::math::*;
use crate::surface::SurfaceModel;
use crate::*;

/// Simple analytic description of an edge curve, used to detect iso-lines on analytic
/// surfaces without numerical projection.
#[derive(Debug, Clone, Copy)]
pub enum CurveHint {
  Line {
    origin: Vec3<f64>,
    dir: Vec3<f64>,
  },
  /// Circle (`rx == ry`) or ellipse.
  Conic {
    frame: Frame,
    rx: f64,
    ry: f64,
  },
  Other,
}

/// The exact 3D geometry of an edge restricted to its vertices.
#[derive(Debug, Clone)]
pub struct Edge3 {
  pub edge: EdgeId,
  /// Ordered from the edge start vertex to the edge end vertex (edge direction, independent of
  /// `same_sense`). First point equals the start vertex and last point the end vertex within
  /// `ctx.tol`, and are snapped exactly to the vertex positions.
  pub segments: Vec<RationalBezierCurve3>,
  pub hint: CurveHint,
  /// Start and end vertex are the same vertex (full circle, closed spline...).
  pub closed: bool,
}

/// The edge's 3D geometry from start vertex to end vertex. Vertices may be up to `10·ctx.tol`
/// away from the curve; the chain end points are snapped exactly onto them and consecutive
/// segments share their joints bitwise.
pub fn build_edge3(model: &BrepModel, edge: EdgeId, ctx: &ConvertCtx) -> ConvertResult<Edge3> {
  edge3::build(model, edge, ctx)
}

/// Per conversion cache, every edge is built once and shared by its two faces.
#[derive(Default)]
pub struct Edge3Cache {
  pub edges: FastHashMap<EdgeId, ConvertResult<Edge3>>,
}

impl Edge3Cache {
  pub fn get(
    &mut self,
    model: &BrepModel,
    edge: EdgeId,
    ctx: &ConvertCtx,
  ) -> ConvertResult<&Edge3> {
    self
      .edges
      .entry(edge)
      .or_insert_with(|| build_edge3(model, edge, ctx))
      .as_ref()
      .map_err(|e| e.clone())
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsoLine {
  /// Constant native `u`.
  U(f64),
  /// Constant native `v`.
  V(f64),
}

/// An edge pcurve in the surface's native parameter space, restricted to the edge and ordered
/// from the edge start vertex to the edge end vertex.
#[derive(Debug, Clone)]
pub struct Edge2 {
  pub segments: Vec<RationalBezierCurve2>,
  /// Set when the pcurve is exactly an iso-line of the surface.
  pub iso: Option<IsoLine>,
}

/// Build the pcurve of `edge` on `surface`. The restriction to the edge is found by matching
/// `surface.eval(pcurve(s))` against the 3D vertices (robust against periodic ambiguity in uv).
///
/// Nothing is snapped in uv. When the pcurve is an iso-line its constant coordinate is made
/// bitwise identical in all control points. Closed edges get the full closed pcurve (pcurves
/// that close only modulo a surface period are continued by that period), the direction is
/// chosen by comparing interior points of the image with `edge3`. Vertices and interior points
/// farther than `10·ctx.tol` from the pcurve image are errors. A degenerate `edge3` (no
/// segments) yields an empty `Edge2`.
pub fn build_edge_pcurve(
  model: &BrepModel,
  edge: EdgeId,
  pcurve: Curve2Id,
  surface: &dyn SurfaceModel,
  edge3: &Edge3,
  ctx: &ConvertCtx,
) -> ConvertResult<Edge2> {
  pcurve::build(model, edge, pcurve, surface, edge3, ctx)
}

/// A full curve (or the given native range of it) as Bézier segments with the native parameter
/// map, segment `i` covering `map.breaks[i]..map.breaks[i+1]` exactly. Used by swept surfaces
/// (extrusion, revolution) where the curve parameter becomes a surface parameter.
#[derive(Debug, Clone)]
pub struct CurveParamSpace {
  pub map: PiecewiseParamMap,
  pub segments: Vec<RationalBezierCurve3>,
}

/// `native_range` is required for unbounded curves (lines), optional otherwise (defaults to the
/// natural domain, one period for periodic curves).
///
/// Without a range, closed curves (conics, chains closing within `ctx.tol`) get a periodic map.
/// With a range: periodic curves accept any start and spans up to one period (the map is
/// periodic when the span is exactly one period); bounded curves clamp the range to their
/// domain (error when nothing is left).
pub fn build_curve_param_space(
  model: &BrepModel,
  curve: CurveId,
  native_range: Option<(f64, f64)>,
  ctx: &ConvertCtx,
) -> ConvertResult<CurveParamSpace> {
  let resolved = resolve(&Curves3(model), curve.index(), Some(ctx.tol))?;
  let chain = match native_range {
    Some((a, b)) => {
      if !(a.is_finite() && b.is_finite() && a < b) {
        return Err(ConvertError::invalid(
          None,
          format!("invalid curve range [{a}, {b}]"),
        ));
      }
      if let Some(period) = resolved.period()
        && b - a > period.length * (1. + 1e-9)
      {
        return Err(ConvertError::invalid(
          None,
          format!(
            "curve range [{a}, {b}] exceeds the period {}",
            period.length
          ),
        ));
      }
      if let Resolved::Chain(chain) = &resolved
        && chain.period.is_none()
      {
        let (d0, d1) = chain.domain();
        if b.min(d1) <= a.max(d0) {
          return Err(ConvertError::invalid(
            None,
            format!("curve range [{a}, {b}] outside the domain [{d0}, {d1}]"),
          ));
        }
      }
      resolved.restrict(a, b)
    }
    None => match resolved {
      Resolved::Chain(chain) => chain,
      Resolved::Line { .. } => {
        return Err(ConvertError::invalid(
          None,
          "LINE parameter space needs a native range",
        ));
      }
    },
  };
  let (map, segments) = chain.to_param_space();
  Ok(CurveParamSpace { map, segments })
}

/// Evaluate a 3D curve at native parameter `t`. Lines, conics, polylines and B-splines (inside
/// their domain) are evaluated directly; other curves through their exact chain.
pub fn eval_curve(model: &BrepModel, curve: CurveId, t: f64) -> ConvertResult<Vec3<f64>> {
  if !t.is_finite() {
    return Err(ConvertError::invalid(None, "non finite curve parameter"));
  }
  let direct = match model.curves.get(curve.index()) {
    Some(CurveGeom::Line { origin, vector }) => Some(*origin + *vector * t),
    Some(CurveGeom::Circle { frame, radius }) => {
      Some(frame.origin + frame.x * (radius * t.cos()) + frame.y * (radius * t.sin()))
    }
    Some(CurveGeom::Ellipse {
      frame,
      semi_axis1,
      semi_axis2,
    }) => Some(frame.origin + frame.x * (semi_axis1 * t.cos()) + frame.y * (semi_axis2 * t.sin())),
    Some(CurveGeom::Polyline(points)) if points.len() >= 2 => {
      let i = (t.floor().max(0.) as usize).min(points.len() - 2);
      let f = t - i as f64;
      Some(points[i] * (1. - f) + points[i + 1] * f)
    }
    _ => None,
  };
  if let Some(p) = direct {
    return Ok(p);
  }
  if let Some(CurveGeom::BSpline(data)) = model.curves.get(curve.index())
    && let Some(p) = eval_bspline_in_domain(data, t)
  {
    return Ok(p);
  }
  let resolved = resolve(&Curves3(model), curve.index(), model_close_tol(model))?;
  Ok(resolved.eval(t))
}

/// Native parameter of the point on the curve closest to `p`, with the distance. Periodic
/// curves return a parameter in their base period. Lines and circles use closed forms.
pub fn project_on_curve(
  model: &BrepModel,
  curve: CurveId,
  p: Vec3<f64>,
) -> ConvertResult<(f64, f64)> {
  let resolved = resolve(&Curves3(model), curve.index(), model_close_tol(model))?;
  Ok(resolved.project(p))
}

fn model_close_tol(model: &BrepModel) -> Option<f64> {
  model.uncertainty.filter(|t| t.is_finite() && *t > 0.)
}

fn eval_bspline_in_domain(data: &BSplineCurveData<Vec3<f64>>, t: f64) -> Option<Vec3<f64>> {
  let weights = data.weights.as_deref();
  if weights.is_some_and(|w| w.len() != data.control_points.len()) {
    return None;
  }
  let control_points = data
    .control_points
    .iter()
    .enumerate()
    .map(|(i, p)| Vec4::from_euclid(*p, weights.map_or(1., |w| w[i])))
    .collect();
  let nurbs = NurbsCurve::new(data.degree, data.knots.clone(), control_points).ok()?;
  let (d0, d1) = nurbs.domain();
  (d0 <= t && t <= d1).then(|| nurbs.eval(t))
}
