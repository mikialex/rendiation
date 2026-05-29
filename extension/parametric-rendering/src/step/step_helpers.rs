use rendiation_step_reader::entities::{Axis2Placement3d, CartesianPoint, Direction};

use crate::*;

pub fn cartesian_point_to_vec3(p: &CartesianPoint) -> Vec3<f32> {
  let coords = &p.coordinates;
  match coords.len() {
    3 => Vec3::new(coords[0] as f32, coords[1] as f32, coords[2] as f32),
    2 => Vec3::new(coords[0] as f32, coords[1] as f32, 0.0),
    1 => Vec3::new(coords[0] as f32, 0.0, 0.0),
    _ => Vec3::zero(),
  }
}

pub fn cartesian_point_to_vec2(p: &CartesianPoint) -> Vec2<f32> {
  let coords = &p.coordinates;
  match coords.len() {
    2 => Vec2::new(coords[0] as f32, coords[1] as f32),
    1 => Vec2::new(coords[0] as f32, 0.0),
    _ if coords.len() >= 3 => Vec2::new(coords[0] as f32, coords[1] as f32),
    _ => Vec2::zero(),
  }
}

pub fn direction_to_vec3(d: &Direction) -> Vec3<f32> {
  let ratios = &d.direction_ratios;
  match ratios.len() {
    3 => Vec3::new(ratios[0] as f32, ratios[1] as f32, ratios[2] as f32),
    2 => Vec3::new(ratios[0] as f32, ratios[1] as f32, 0.0),
    1 => Vec3::new(ratios[0] as f32, 0.0, 0.0),
    _ => Vec3::zero(),
  }
}

/// Get the X-direction from an Axis2Placement3D, or compute a default
/// perpendicular to the axis when ref_direction is `$` (None).
pub fn axis2_x_dir(position: &Axis2Placement3d) -> Vec3<f32> {
  let axis = direction_to_vec3(&position.axis).normalize();
  let x = if let Some(ref_dir) = &position.ref_direction {
    direction_to_vec3(ref_dir)
  } else if axis.x.abs() < 0.9 {
    Vec3::new(1.0, 0.0, 0.0)
  } else {
    Vec3::new(0.0, 1.0, 0.0)
  };
  // Orthogonalize x against the axis (same as truck)
  (x - x.dot(axis) * axis).normalize()
}
