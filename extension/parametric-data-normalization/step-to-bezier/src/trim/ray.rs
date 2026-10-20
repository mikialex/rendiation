//! Decide whether the boundary of a cell without boundary events lies in the material, by a
//! first-hit ray test against all loop curves (periodic copies included).

use std::f64::consts::{FRAC_PI_2, PI};

use rendiation_algebra::*;

use super::cell::*;
use super::chain::*;
use super::geom::*;
use crate::math::*;
use crate::*;

/// Hits closer than this to a curve end (in curve parameter) are vertex hits.
const VERTEX_T_EPS: f64 = 1e-7;
/// Minimum `|sin|` of the angle between ray and curve tangent at a usable hit.
const MIN_HIT_SIN: f64 = 0.02;
/// Fractions along each edge tried as reference points, edge midpoints first.
const REFERENCE_FRACTIONS: [f64; 7] = [0.5, 0.31, 0.69, 0.137, 0.863, 0.42, 0.58];
/// Ray directions as angles relative to the inward edge normal. Axis aligned ones first (cheap
/// thin bounding boxes), then odd angles unlikely to align with curve features.
const RAY_ANGLES: [f64; 14] = [
  0.,
  PI,
  FRAC_PI_2,
  -FRAC_PI_2,
  0.4137,
  -0.4137,
  0.9271,
  -0.9271,
  1.3093,
  -1.3093,
  PI - 0.5361,
  PI + 0.5361,
  0.1739,
  -0.1739,
];

enum RayResult {
  /// First hit is usable: whether the ray origin is on the material side.
  Hit(bool),
  /// The origin lies on a curve.
  OnCurve,
  /// First hit is at a vertex, tangential, or ambiguous.
  Degenerate,
  Miss,
}

pub(super) fn boundary_in_material(
  cell: &Cell,
  loops: &[PreparedLoop],
  periods: [Option<f64>; 2],
) -> ConvertResult<bool> {
  if loops.is_empty() {
    return Ok(true);
  }
  let length = ray_length(cell, loops, periods);
  let on_curve_eps = 16. * cell.eps + 1e-12;
  let mut found_free_point = false;
  for fraction in REFERENCE_FRACTIONS {
    for edge in 0..4 {
      let origin = cell.corner(edge) + Cell::edge_direction(edge) * fraction;
      let inward = Cell::edge_direction(edge + 1);
      let mut all_miss = true;
      let mut on_curve = false;
      for angle in RAY_ANGLES {
        let (sin, cos) = angle.sin_cos();
        let dir = if angle == 0. {
          inward
        } else {
          V2::new(
            inward.x * cos - inward.y * sin,
            inward.x * sin + inward.y * cos,
          )
        };
        match cast_ray(origin, dir, length, loops, periods, on_curve_eps) {
          RayResult::Hit(inside) => return Ok(inside),
          RayResult::OnCurve => {
            on_curve = true;
            break;
          }
          RayResult::Degenerate => all_miss = false,
          RayResult::Miss => {}
        }
      }
      if on_curve {
        continue;
      }
      found_free_point = true;
      if all_miss {
        return Ok(unbounded_region_is_material(loops));
      }
    }
  }
  if !found_free_point {
    // Every reference point lies on a curve: the cell boundary is covered by curves. Kept ones
    // are part of inside cycles already, dropped ones have their material outside the cell.
    return Ok(false);
  }
  Err(ConvertError::numerical(
    None,
    format!(
      "trim: cell {:?}: could not classify the cell boundary, every ray hit was degenerate",
      cell.index
    ),
  ))
}

/// Material status of points no ray reaches (winding number zero with respect to every loop):
/// material only when the region is the complement of its loops (holes only, negative total
/// area) or there are no loops at all. Band loops bound the region along their periodic axis,
/// so with band loops present unreachable points are outside.
fn unbounded_region_is_material(loops: &[PreparedLoop]) -> bool {
  if loops.iter().any(|l| l.is_band()) {
    return false;
  }
  let area: f64 = loops
    .iter()
    .map(|l| loop_signed_area(l.curves.iter().map(|c| &c.curve), 2))
    .sum();
  area < 0.
}

/// Long enough to reach every loop copy that can be the first hit: the extent of all loops and
/// the cell, plus a few periods.
fn ray_length(cell: &Cell, loops: &[PreparedLoop], periods: [Option<f64>; 2]) -> f64 {
  let world = loops.iter().fold(cell.bbox(), |b, l| bbox_union(b, l.bbox));
  let extent = (world.1 - world.0).length();
  let period_sum: f64 = periods.iter().flatten().sum();
  2. * (extent + 2. * period_sum + 2.)
}

fn cast_ray(
  origin: V2,
  dir: V2,
  length: f64,
  loops: &[PreparedLoop],
  periods: [Option<f64>; 2],
  on_curve_eps: f64,
) -> RayResult {
  let end = origin + dir * length;
  let segment = bbox_expand(
    (
      V2::new(origin.x.min(end.x), origin.y.min(end.y)),
      V2::new(origin.x.max(end.x), origin.y.max(end.y)),
    ),
    on_curve_eps,
  );
  let normal = V2::new(-dir.y, dir.x);
  // (distance along the ray, curve parameter, tangent)
  let mut hits = Vec::new();
  for l in loops {
    let Some([ru, rv]) = lattice_ranges(l.bbox, segment, periods) else {
      continue;
    };
    for ku in ru {
      for kv in rv.clone() {
        let offset = lattice_offset([ku, kv], periods);
        // Intersect the untranslated curves with the ray shifted by `-offset`.
        let q = origin - offset;
        for (c, bbox) in l.curves.iter().zip(&l.bboxes) {
          if !bbox_overlaps(bbox_translate(*bbox, offset), segment) {
            continue;
          }
          line_hits(&c.curve, q, dir, normal, length, on_curve_eps, &mut hits);
        }
      }
    }
  }
  let Some(nearest) = hits.iter().min_by(|a, b| a.0.total_cmp(&b.0)) else {
    return RayResult::Miss;
  };
  let (s, t, tangent) = *nearest;
  if s <= on_curve_eps {
    return RayResult::OnCurve;
  }
  let second = hits
    .iter()
    .filter(|h| !std::ptr::eq(*h, nearest))
    .map(|h| h.0)
    .fold(f64::INFINITY, f64::min);
  let tangent_len = tangent.length();
  if !(VERTEX_T_EPS..=1. - VERTEX_T_EPS).contains(&t)
    || second - s <= on_curve_eps
    || tangent_len.is_nan()
    || tangent_len <= 0.
    || cross(dir, tangent).abs() < MIN_HIT_SIN * tangent_len
  {
    return RayResult::Degenerate;
  }
  // Material is on the left of the tangent; the origin lies behind the hit (direction -dir).
  RayResult::Hit(cross(tangent, dir * -1.) > 0.)
}

fn line_hits(
  curve: &RationalBezierCurve2,
  origin: V2,
  dir: V2,
  normal: V2,
  length: f64,
  on_curve_eps: f64,
  hits: &mut Vec<(f64, f64, V2)>,
) {
  let coeffs: Vec<_> = curve
    .control_points
    .iter()
    .map(|p| normal.x * (p.x - origin.x * p.z) + normal.y * (p.y - origin.y * p.z))
    .collect();
  let scale = curve
    .control_points
    .iter()
    .map(|p| {
      normal.x.abs() * (p.x.abs() + (origin.x * p.z).abs())
        + normal.y.abs() * (p.y.abs() + (origin.y * p.z).abs())
    })
    .fold(0., f64::max);
  for t in bernstein_roots(&coeffs, scale * 1e-12, 1e-12) {
    let (p, d) = curve.eval_with_derivative(t);
    let s = (p - origin).dot(dir);
    if s >= -on_curve_eps && s <= length {
      hits.push((s, t, d));
    }
  }
}
