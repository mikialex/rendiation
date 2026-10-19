//! Edge 3D geometry restricted to the edge vertices.

use super::chain::*;
use super::resolve::*;
use super::*;

/// Vertices farther than this many `ctx.tol` from their curve are rejected; closer ones are
/// accepted and the chain end points snapped onto them.
pub(super) const VERTEX_DISTANCE_FACTOR: f64 = 10.;

pub(super) fn build(model: &BrepModel, edge_id: EdgeId, ctx: &ConvertCtx) -> ConvertResult<Edge3> {
  let edge = model
    .edges
    .get(edge_id.index())
    .ok_or_else(|| ConvertError::invalid(None, format!("edge id {} out of range", edge_id.0)))?;
  build_edge(model, edge_id, edge, ctx).map_err(|e| e.at(edge.step_id))
}

fn build_edge(
  model: &BrepModel,
  edge_id: EdgeId,
  edge: &Edge,
  ctx: &ConvertCtx,
) -> ConvertResult<Edge3> {
  let position = |v: VertexId| {
    model
      .vertices
      .get(v.index())
      .map(|v| v.position)
      .filter(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
      .ok_or_else(|| ConvertError::invalid(None, format!("vertex {} missing or invalid", v.0)))
  };
  let start = position(edge.start)?;
  let end = position(edge.end)?;
  let closed = edge.start == edge.end;
  let resolved = resolve(&Curves3(model), edge.curve.index(), Some(ctx.tol))?;

  let mut out = Edge3 {
    edge: edge_id,
    segments: Vec::new(),
    hint: CurveHint::Other,
    closed,
  };
  let line = match &resolved {
    Resolved::Line { origin, vector } => Some((*origin, *vector)),
    Resolved::Chain(chain) => match chain.analytic {
      Analytic::Line { origin, vector } => Some((origin, vector)),
      _ => None,
    },
  };

  if let Some((origin, vector)) = line {
    // The segment between the vertices is the whole truth for a line; only check that the
    // vertices lie on the line.
    let line = Resolved::<Vec4<f64>>::Line { origin, vector };
    check_vertex_distance(line.project(start).1, "start", ctx)?;
    check_vertex_distance(line.project(end).1, "end", ctx)?;
    let dir = if edge.same_sense {
      vector
    } else {
      vector * -1.
    };
    out.hint = CurveHint::Line {
      origin: start,
      dir: dir.normalize(),
    };
    if !closed && (end - start).length() > ctx.tol {
      out.hint = CurveHint::Line {
        origin: start,
        dir: (end - start).normalize(),
      };
      out.segments.push(RationalBezierCurve::line(start, end));
    }
    return Ok(out);
  }

  let Resolved::Chain(chain) = resolved else {
    return Err(ConvertError::numerical(
      None,
      "line edge not handled as line",
    ));
  };
  if let Analytic::Conic(conic) = chain.analytic {
    out.hint = CurveHint::Conic {
      frame: Frame {
        origin: conic.center,
        x: conic.x,
        y: conic.y,
        z: conic.x.cross(conic.y),
      },
      rx: conic.rx,
      ry: conic.ry,
    };
  }

  let (s0, d0) = chain.project(start);
  let (s1, d1) = chain.project(end);
  check_vertex_distance(d0, "start", ctx)?;
  check_vertex_distance(d1, "end", ctx)?;
  // Distinct vertices at the same place on a closed curve are treated as a full loop.
  let full = closed || (end - start).length() <= ctx.tol;

  let interval = match chain.period {
    Some(period) => {
      let length = period.length;
      if full {
        if edge.same_sense {
          (s0, s0 + length)
        } else {
          (s0 - length, s0)
        }
      } else if edge.same_sense {
        (s0, s0 + (s1 - s0).rem_euclid(length))
      } else {
        (s0 - (s0 - s1).rem_euclid(length), s0)
      }
    }
    None => {
      if full {
        // An open curve can not carry a loop edge: zero length (degenerate) edge.
        return Ok(out);
      }
      let (lo, hi) = if edge.same_sense { (s0, s1) } else { (s1, s0) };
      if lo >= hi {
        return Err(ConvertError::invalid(
          None,
          format!(
            "edge vertices at curve parameters {s0} -> {s1} contradict same_sense {}",
            edge.same_sense
          ),
        ));
      }
      (lo, hi)
    }
  };
  if interval.0 >= interval.1 {
    return Ok(out);
  }

  let mut segments = chain.restrict(interval.0, interval.1).curves();
  if !edge.same_sense {
    segments.reverse();
    for s in &mut segments {
      *s = s.reversed();
    }
  }
  snap_chain(&mut segments, start, end);
  out.segments = segments;
  Ok(out)
}

fn check_vertex_distance(distance: f64, which: &str, ctx: &ConvertCtx) -> ConvertResult<()> {
  if distance.is_nan() || distance > VERTEX_DISTANCE_FACTOR * ctx.tol {
    return Err(ConvertError::invalid(
      None,
      format!(
        "edge {which} vertex is {distance} away from its curve (tol {})",
        ctx.tol
      ),
    ));
  }
  Ok(())
}

/// Snap the chain ends exactly onto the vertices and make consecutive segments share their
/// joint points bitwise.
fn snap_chain(segments: &mut [RationalBezierCurve3], start: Vec3<f64>, end: Vec3<f64>) {
  let n = segments.len();
  for i in 0..n {
    let s = if i == 0 { start } else { segments[i - 1].end() };
    let e = if i + 1 == n { end } else { segments[i].end() };
    segments[i] = segments[i].with_endpoints(s, e);
  }
}
