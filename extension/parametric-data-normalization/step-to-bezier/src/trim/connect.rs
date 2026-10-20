//! Assemble open fragments into closed loops by walking the cell boundary counter-clockwise from
//! each exit to the next entry.

use rendiation_algebra::*;

use super::cell::*;
use super::geom::*;
use crate::math::*;
use crate::*;

/// A point where a fragment starts (entry) or ends (exit) on the cell boundary.
#[derive(Debug, Clone, Copy)]
struct BoundaryEvent {
  point: V2,
  s: f64,
  /// Boundary angle (see `Cell::boundary_angle`) of the fragment direction at the event,
  /// probed at a tiny distance along the fragment (forwards for entries, backwards for exits).
  /// It orders events sharing one boundary point along an infinitesimal arc inside the cell.
  /// Probing a chord instead of using the tangent resolves tangential ties by curvature (for
  /// example an arc leaving a corner tangentially along the edge another fragment arrived on).
  angle: f64,
}

pub(super) fn connect_fragments(
  cell: &Cell,
  fragments: Vec<Vec<TrimCurve>>,
) -> ConvertResult<Vec<Vec<TrimCurve>>> {
  let mut entries = Vec::with_capacity(fragments.len());
  let mut exits = Vec::with_capacity(fragments.len());
  for curves in &fragments {
    let (Some(first), Some(last)) = (curves.first(), curves.last()) else {
      return Err(ConvertError::topology(None, "trim: empty fragment"));
    };
    let (start, end) = (first.curve.start(), last.curve.end());
    entries.push(event(cell, start, probe(start, curves.iter(), false))?);
    exits.push(event(cell, end, probe(end, curves.iter().rev(), true))?);
  }
  let n = fragments.len();
  let mut fragments: Vec<_> = fragments.into_iter().map(Some).collect();
  let mut loops = Vec::new();
  for first in 0..n {
    let mut out = Vec::new();
    let mut current = first;
    let mut closed = false;
    for _ in 0..n {
      let Some(curves) = fragments[current].take() else {
        break;
      };
      out.extend(curves);
      let exit = exits[current];
      let (next, distance) = next_entry(&exit, &entries, cell.eps);
      let entry = entries[next];
      out.extend(boundary_walk(
        cell,
        exit.point,
        exit.s,
        entry.point,
        distance,
      ));
      if next == first {
        closed = true;
        break;
      }
      if fragments[next].is_none() {
        return Err(unmatched(cell, &exit, &entry));
      }
      current = next;
    }
    if closed {
      loops.push(out);
    } else if !out.is_empty() {
      return Err(ConvertError::topology(
        None,
        format!(
          "trim: cell {:?}: boundary walk from fragment {first} did not close",
          cell.index
        ),
      ));
    }
  }
  Ok(loops)
}

fn event(cell: &Cell, point: V2, direction: V2) -> ConvertResult<BoundaryEvent> {
  let Some(s) = cell.boundary_param(point) else {
    return Err(ConvertError::topology(
      None,
      format!(
        "trim: cell {:?}: fragment end ({}, {}) is not on the cell boundary",
        cell.index, point.x, point.y
      ),
    ));
  };
  Ok(BoundaryEvent {
    point,
    s,
    angle: Cell::boundary_angle(s, direction),
  })
}

fn unmatched(cell: &Cell, exit: &BoundaryEvent, entry: &BoundaryEvent) -> ConvertError {
  ConvertError::topology(
    None,
    format!(
      "trim: cell {:?}: exit at ({}, {}) connects to the entry at ({}, {}) which is already \
       used, entries and exits do not match (inconsistent loop orientation?)",
      cell.index, exit.point.x, exit.point.y, entry.point.x, entry.point.y
    ),
  )
}

/// The entry reached first when walking counter-clockwise from `exit`, and the walked boundary
/// parameter distance (`0` for no walk, about `4` for a full turn back to the same point).
///
/// Events closer than `eps` along the boundary are at the same point. They are ordered along an
/// infinitesimal arc inside the cell, which the counter-clockwise walk traverses from larger to
/// smaller boundary angles: an entry at the exit point comes before the walk when its angle is
/// below the exit angle, otherwise it is reached after a full turn. Equal angles (within
/// `ANGLE_TIE`) mean the entry
/// runs back along the exit (two regions sharing that curve): the entry's material is on the
/// larger angle side and the exit's on the smaller one, so the walk comes first.
fn next_entry(exit: &BoundaryEvent, entries: &[BoundaryEvent], eps: f64) -> (usize, f64) {
  let key = |e: &BoundaryEvent| {
    let mut d = e.s - exit.s;
    if d < 0. {
      d += 4.;
    }
    let near = if d <= eps {
      Some(d)
    } else if d >= 4. - eps {
      Some(d - 4.)
    } else {
      None
    };
    let (major, distance) = match near {
      Some(_) if e.angle < exit.angle - ANGLE_TIE => (0., 0.),
      Some(signed) => (4., 4. + signed),
      None => (d, d),
    };
    (major, -e.angle, distance)
  };
  let mut best = 0;
  let mut best_key = key(&entries[0]);
  for (k, e) in entries.iter().enumerate().skip(1) {
    let candidate = key(e);
    if candidate.0 < best_key.0 || (candidate.0 == best_key.0 && candidate.1 < best_key.1) {
      best = k;
      best_key = candidate;
    }
  }
  (best, best_key.2)
}

/// Distance along a fragment at which its direction is probed.
const ANGLE_PROBE: f64 = 1e-5;
/// Probe angles closer than this are equal (curves running along each other).
const ANGLE_TIE: f64 = 1e-10;

/// Vector from `q` to the first point of the curve chain at distance `ANGLE_PROBE` from `q`
/// (curves traversed backwards when `reverse`), or to the chain end if it never gets that far.
///
/// The offset is evaluated from control point differences relative to `q`, so its rounding
/// error is relative to the curve size, not to the magnitude of the coordinates: curves running
/// exactly along each other produce exactly equal angles.
fn probe<'a>(q: V2, curves: impl Iterator<Item = &'a TrimCurve>, reverse: bool) -> V2 {
  let mut last = V2::new(0., 0.);
  let mut relative = Vec::new();
  for c in curves {
    relative.clear();
    relative.extend(
      c.curve
        .control_points
        .iter()
        .map(|p| Vec3::new(p.x - q.x * p.z, p.y - q.y * p.z, p.z)),
    );
    if reverse {
      relative.reverse();
    }
    let at = |t: f64| de_casteljau(&relative, t).to_euclid();
    let reached = |t: f64| at(t).length() >= ANGLE_PROBE;
    if !reached(1.) {
      last = at(1.);
      continue;
    }
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..64 {
      let mid = 0.5 * hi;
      if reached(mid) {
        hi = mid;
      } else {
        lo = mid;
        break;
      }
    }
    for _ in 0..64 {
      let mid = 0.5 * (lo + hi);
      if reached(mid) {
        hi = mid;
      } else {
        lo = mid;
      }
    }
    return at(hi);
  }
  last
}

/// Straight `PatchBoundary` lines from `a` (boundary parameter `sa`) counter-clockwise over
/// `distance` to `b`, split at the cell corners, zero length segments skipped.
fn boundary_walk(cell: &Cell, a: V2, sa: f64, b: V2, distance: f64) -> Vec<TrimCurve> {
  let mut points = vec![a];
  let mut k = sa.floor() + 1.;
  while k < sa + distance {
    points.push(cell.corner(k as usize));
    k += 1.;
  }
  points.push(b);
  lines(&points)
}

/// The full cell boundary, counter-clockwise from corner 0.
pub(super) fn rectangle_loop(cell: &Cell) -> Vec<TrimCurve> {
  let points: Vec<_> = (0..=4).map(|k| cell.corner(k)).collect();
  lines(&points)
}

fn lines(points: &[V2]) -> Vec<TrimCurve> {
  points
    .windows(2)
    .filter(|w| w[0] != w[1])
    .map(|w| TrimCurve {
      curve: RationalBezierCurve2::line(w[0], w[1]),
      source: TrimCurveSource::PatchBoundary,
      exact: true,
    })
    .collect()
}
