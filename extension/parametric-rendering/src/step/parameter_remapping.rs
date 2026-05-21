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
