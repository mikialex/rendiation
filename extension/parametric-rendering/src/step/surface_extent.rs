use super::*;
use crate::*;

/// Compute the U,V extent of a plane face by projecting edge curve points
/// onto the plane's local coordinate system.
///
/// Returns (u_min, u_max, v_min, v_max).
/// Falls back to (-1, 1, -1, 1) if no points can be obtained.
pub fn compute_plane_face_extent(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edge_loops: &[Vec<EdgeData>],
  config: &StepReadConfig,
) -> (f32, f32, f32, f32) {
  let origin = cartesian_point_to_vec3(&position.location);
  let normal = direction_to_vec3(&position.axis);
  let u_dir = axis2_x_dir(position);
  let v_dir = normal.cross(u_dir).normalize();

  let mut u_min = f32::MAX;
  let mut u_max = f32::MIN;
  let mut v_min = f32::MAX;
  let mut v_max = f32::MIN;
  let mut any_point = false;

  for edge in edge_loops.iter().flat_map(|l| l.iter()) {
    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      for p in &pts {
        let local = *p - origin;
        let u = local.dot(u_dir);
        let v = local.dot(v_dir);
        u_min = u_min.min(u);
        u_max = u_max.max(u);
        v_min = v_min.min(v);
        v_max = v_max.max(v);
        any_point = true;
      }
    }
  }

  if !any_point {
    return (-1.0, 1.0, -1.0, 1.0);
  }

  (u_min, u_max, v_min, v_max)
}

/// Compute the V-axis extent for cylinder/cone faces by projecting edge
/// curve points onto the surface's axis direction.
///
/// V is the signed distance from the placement origin along the axis.
/// Returns (v_min, v_max). Falls back to (0.0, 1.0)
/// if no points can be obtained.
pub fn compute_axis_v_extent(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edge_loops: &[Vec<EdgeData>],
  config: &StepReadConfig,
) -> (f64, f64) {
  let origin = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();

  let mut v_min = f64::MAX;
  let mut v_max = f64::MIN;
  let mut any_point = false;

  for edge in edge_loops.iter().flat_map(|l| l.iter()) {
    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      for p in &pts {
        let v = (*p - origin).dot(axis) as f64;
        v_min = v_min.min(v);
        v_max = v_max.max(v);
        any_point = true;
      }
    }
  }

  if !any_point {
    return (0.0, 1.0);
  }

  (v_min, v_max)
}
