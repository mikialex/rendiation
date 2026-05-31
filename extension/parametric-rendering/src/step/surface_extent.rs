use super::*;
use crate::*;

/// Compute the U,V extent of a plane face from pre-converted Bezier edge curves.
pub fn compute_plane_face_extent_from_beziers(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edge_loop_beziers: &[Vec<Vec<RationalBezierCurve3d<f32>>>],
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

  for edge_loop in edge_loop_beziers {
    for beziers in edge_loop {
      for curve in beziers {
        let pts = adaptive_tessellate_bezier_curve(curve, config.trim_curve_tessellate_tolerance);
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
  }

  if !any_point {
    return (-1.0, 1.0, -1.0, 1.0);
  }

  // Guard against zero-span extents (all edge points collinear in one direction).
  let u_span_raw = u_max - u_min;
  let v_span_raw = v_max - v_min;
  let u_span = u_span_raw.max(1e-6);
  let v_span = v_span_raw.max(1e-6);
  if u_span_raw < 1e-6 || v_span_raw < 1e-6 {
    println!(
      "plane face extent near-degenerate: u_span={u_span_raw:e}, v_span={v_span_raw:e}; clamped to 1e-6"
    );
  }
  (u_min, u_min + u_span, v_min, v_min + v_span)
}

/// Compute the V-axis extent for cylinder/cone faces from pre-converted Bezier edge curves.
pub fn compute_axis_v_extent_from_beziers(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edge_loop_beziers: &[Vec<Vec<RationalBezierCurve3d<f32>>>],
  config: &StepReadConfig,
) -> (f64, f64) {
  let origin = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();

  let mut v_min = f64::MAX;
  let mut v_max = f64::MIN;
  let mut any_point = false;

  for edge_loop in edge_loop_beziers {
    for beziers in edge_loop {
      for curve in beziers {
        let pts = adaptive_tessellate_bezier_curve(curve, config.trim_curve_tessellate_tolerance);
        for p in &pts {
          let v = (*p - origin).dot(axis) as f64;
          v_min = v_min.min(v);
          v_max = v_max.max(v);
          any_point = true;
        }
      }
    }
  }

  if !any_point {
    return (0.0, 1.0);
  }

  (v_min, v_max)
}
