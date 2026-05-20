use super::*;
use crate::*;

/// Remap a set of 2D points from the original surface parameter space to a
/// patch's local `[0, 1]²` parameter domain using an affine transformation.
///
/// For B-spline → Bezier decomposition, each patch covers exactly one knot span,
/// so the mapping from the original (u, v) to the patch-local (û, v̂) is:
///
/// ```text
///   û = (u - u_patch_min) / (u_patch_max - u_patch_min)
///   v̂ = (v - v_patch_min) / (v_patch_max - v_patch_min)
/// ```
///
/// Points outside the patch domain are dropped.
pub fn remap_2d_points_to_patch(points: &[Vec2<f32>], range: &SubRange) -> Vec<Vec2<f32>> {
  let du = range.u_range.1 - range.u_range.0;
  let dv = range.v_range.1 - range.v_range.0;

  if du.abs() < 1e-10 || dv.abs() < 1e-10 {
    return Vec::new();
  }

  points
    .iter()
    .filter_map(|p| {
      if p.x >= range.u_range.0
        && p.x <= range.u_range.1
        && p.y >= range.v_range.0
        && p.y <= range.v_range.1
      {
        let u_local = (p.x - range.u_range.0) / du;
        let v_local = (p.y - range.v_range.0) / dv;
        Some(Vec2::new(u_local.clamp(0.0, 1.0), v_local.clamp(0.0, 1.0)))
      } else {
        None
      }
    })
    .collect()
}

/// Given per-edge 2D polylines projected onto a single Bezier patch, insert
/// boundary-edge polyline points so that open endpoints on the `[0,1]²` boundary
/// are connected without cutting through the patch interior.
///
/// Returns one polyline per connected component (typically one outer loop).
///
/// The `[0,1]²` boundary edges are:
///   edge 0: u=0, v 0→1   (perimeter t=0..1)
///   edge 1: v=1, u 0→1   (perimeter t=1..2)
///   edge 2: u=1, v 1→0   (perimeter t=2..3)
///   edge 3: v=0, u 1→0   (perimeter t=3..4)
pub fn connect_polylines_with_boundary(
  per_edge_polylines: Vec<Vec<Vec2<f32>>>,
  boundary_epsilon: f32,
) -> Vec<Vec<Vec2<f32>>> {
  if per_edge_polylines.is_empty() {
    return Vec::new();
  }

  // Collect all polylines with their endpoints
  let mut polylines: Vec<Vec<Vec2<f32>>> = per_edge_polylines
    .into_iter()
    .filter(|p| !p.is_empty())
    .collect();

  if polylines.is_empty() {
    return Vec::new();
  }

  // Build endpoint graph: map endpoint → list of (polyline_index, is_start)
  let mut endpoints: Vec<(Vec2<f32>, Vec<(usize, bool)>)> = Vec::new();

  let snap = |p: Vec2<f32>| -> Vec2<f32> { Vec2::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)) };

  for (pi, poly) in polylines.iter().enumerate() {
    let first = snap(poly[0]);
    let last = snap(poly[poly.len() - 1]);

    // If polyline is essentially a closed loop, skip endpoint handling
    if (first - last).length() < boundary_epsilon {
      continue;
    }

    // Add or merge start endpoint
    add_endpoint(&mut endpoints, first, pi, true, boundary_epsilon);
    // Add or merge end endpoint
    add_endpoint(&mut endpoints, last, pi, false, boundary_epsilon);
  }

  // Find singleton endpoints (degree 1 — need boundary connection)
  let singletons: Vec<Vec2<f32>> = endpoints
    .iter()
    .filter(|(_, refs)| refs.len() == 1)
    .map(|(pos, _)| *pos)
    .collect();

  if singletons.is_empty() {
    // All polylines are already closed or connected — concatenate and return
    return vec![concatenate_polylines(&polylines)];
  }

  // Snap singletons to boundary, compute perimeter positions
  let mut boundary_points: Vec<(f32, Vec2<f32>)> = singletons
    .iter()
    .filter_map(|&p| snap_to_boundary(p, boundary_epsilon))
    .collect();

  if boundary_points.is_empty() {
    return vec![concatenate_polylines(&polylines)];
  }

  // Sort boundary points by perimeter position
  boundary_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

  // Insert boundary polyline segments between consecutive boundary points
  let boundary_samples = 8;
  for i in 0..boundary_points.len() {
    let (t_a, _p_a) = boundary_points[i];
    let (t_b, _p_b) = boundary_points[(i + 1) % boundary_points.len()];

    // Walk CCW from t_a to t_b along the perimeter
    let mut seg_points = Vec::new();
    let dist = ccw_perimeter_distance(t_a, t_b);
    let n_samples = (dist * boundary_samples as f32).ceil() as usize;
    for s in 0..=n_samples {
      let t = if n_samples == 0 {
        t_a
      } else {
        t_a + (s as f32) * dist / (n_samples as f32)
      };
      seg_points.push(perimeter_to_point(t % 4.0));
    }
    polylines.push(seg_points);
  }

  vec![concatenate_polylines(&polylines)]
}

fn add_endpoint(
  endpoints: &mut Vec<(Vec2<f32>, Vec<(usize, bool)>)>,
  p: Vec2<f32>,
  poly_idx: usize,
  is_start: bool,
  epsilon: f32,
) {
  for (ep, refs) in endpoints.iter_mut() {
    if (*ep - p).length() < epsilon {
      refs.push((poly_idx, is_start));
      return;
    }
  }
  endpoints.push((p, vec![(poly_idx, is_start)]));
}

/// Snap a point to the nearest [0,1]² boundary edge if within epsilon.
/// Returns Some((perimeter_position, snapped_point)).
fn snap_to_boundary(p: Vec2<f32>, epsilon: f32) -> Option<(f32, Vec2<f32>)> {
  let dists = [
    (
      p.x.abs(),
      0,
      Vec2::new(0.0, p.y.clamp(0.0, 1.0)),
      p.y.clamp(0.0, 1.0),
    ), // u=0
    (
      (1.0 - p.y).abs(),
      1,
      Vec2::new(p.x.clamp(0.0, 1.0), 1.0),
      1.0 + p.x.clamp(0.0, 1.0),
    ), // v=1
    (
      (1.0 - p.x).abs(),
      2,
      Vec2::new(1.0, p.y.clamp(0.0, 1.0)),
      2.0 + (1.0 - p.y.clamp(0.0, 1.0)),
    ), // u=1
    (
      p.y.abs(),
      3,
      Vec2::new(p.x.clamp(0.0, 1.0), 0.0),
      3.0 + (1.0 - p.x.clamp(0.0, 1.0)),
    ), // v=0
  ];

  let mut best: Option<(f32, Vec2<f32>, f32)> = None;
  for &(dist, _edge, pt, t) in &dists {
    if dist < epsilon {
      match best {
        None => best = Some((dist, pt, t)),
        Some((bd, _, _)) if dist < bd => best = Some((dist, pt, t)),
        _ => {}
      }
    }
  }
  best.map(|(_, pt, t)| (t, pt))
}

/// CCW distance from perimeter position a to b (b > a, wrapping around).
fn ccw_perimeter_distance(a: f32, b: f32) -> f32 {
  if b >= a {
    b - a
  } else {
    4.0 - a + b
  }
}

/// Convert a perimeter position t ∈ [0, 4) to a [0,1]² point.
fn perimeter_to_point(t: f32) -> Vec2<f32> {
  let t = t.clamp(0.0, 3.999);
  if t < 1.0 {
    Vec2::new(0.0, t) // u=0, v 0→1
  } else if t < 2.0 {
    Vec2::new(t - 1.0, 1.0) // v=1, u 0→1
  } else if t < 3.0 {
    Vec2::new(1.0, 1.0 - (t - 2.0)) // u=1, v 1→0
  } else {
    Vec2::new(1.0 - (t - 3.0), 0.0) // v=0, u 1→0
  }
}

fn concatenate_polylines(polylines: &[Vec<Vec2<f32>>]) -> Vec<Vec2<f32>> {
  let mut result = Vec::new();
  for poly in polylines {
    if result.is_empty() {
      result.extend(poly.iter());
    } else {
      // Skip the first point if it matches the last point (duplicate)
      let skip_dup = match result.last() {
        Some(&lp) => {
          let d: Vec2<f32> = lp - poly[0];
          d.length() < 1e-6
        }
        None => false,
      };
      let start: usize = if skip_dup { 1 } else { 0 };
      result.extend(poly[start..].iter());
    }
  }
  result
}

/// Ray-casting point-in-polygon test for a closed 2D polygon.
///
/// Returns `true` if `point` is inside the polygon (including on edges).
/// The polygon is assumed closed (last point connects to first).
pub fn point_in_polygon(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
  let n = polygon.len();
  if n < 3 {
    return false;
  }
  let mut inside = false;
  let mut j = n - 1;
  for i in 0..n {
    let a = polygon[i];
    let b = polygon[j];
    // Check if the ray from point to +x crosses edge (a,b)
    if (a.y > point.y) != (b.y > point.y) {
      let x_intersect = a.x + (b.x - a.x) * (point.y - a.y) / (b.y - a.y);
      if point.x < x_intersect {
        inside = !inside;
      }
    }
    j = i;
  }
  inside
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_range(u0: f32, u1: f32, v0: f32, v1: f32) -> SubRange {
    SubRange {
      u_range: (u0, u1),
      v_range: (v0, v1),
    }
  }

  #[test]
  fn remap_midpoint() {
    let patch = make_range(0.0, 1.0, 0.0, 1.0);
    let points = vec![Vec2::new(0.5, 0.5)];
    let result = remap_2d_points_to_patch(&points, &patch);
    assert_eq!(result.len(), 1);
    assert!((result[0].x - 0.5).abs() < 1e-6);
    assert!((result[0].y - 0.5).abs() < 1e-6);
  }

  #[test]
  fn remap_offset_patch() {
    let patch = make_range(0.5, 1.0, 0.0, 0.5);
    let points = vec![Vec2::new(0.75, 0.25)];
    let result = remap_2d_points_to_patch(&points, &patch);
    assert_eq!(result.len(), 1);
    assert!((result[0].x - 0.5).abs() < 1e-6); // (0.75-0.5)/(1.0-0.5) = 0.5
    assert!((result[0].y - 0.5).abs() < 1e-6); // (0.25-0.0)/(0.5-0.0) = 0.5
  }

  #[test]
  fn remap_outside_patch_dropped() {
    let patch = make_range(0.0, 0.5, 0.0, 0.5);
    let points = vec![Vec2::new(0.75, 0.25)];
    let result = remap_2d_points_to_patch(&points, &patch);
    assert_eq!(result.len(), 0);
  }

  #[test]
  fn point_in_square() {
    let square = vec![
      Vec2::new(0.0, 0.0),
      Vec2::new(1.0, 0.0),
      Vec2::new(1.0, 1.0),
      Vec2::new(0.0, 1.0),
    ];
    assert!(point_in_polygon(Vec2::new(0.5, 0.5), &square));
    assert!(point_in_polygon(Vec2::new(0.1, 0.1), &square));
    assert!(!point_in_polygon(Vec2::new(1.5, 0.5), &square));
    assert!(!point_in_polygon(Vec2::new(-0.1, 0.5), &square));
  }

  #[test]
  fn point_outside_polygon() {
    let triangle = vec![
      Vec2::new(0.0, 0.0),
      Vec2::new(1.0, 0.0),
      Vec2::new(0.5, 1.0),
    ];
    assert!(point_in_polygon(Vec2::new(0.5, 0.3), &triangle));
    assert!(!point_in_polygon(Vec2::new(0.5, -0.3), &triangle));
    assert!(!point_in_polygon(Vec2::new(1.5, 0.5), &triangle));
  }
}
