use crate::*;

/// target_world_aabb should not empty, is target is unbound, should give it logical center point as
/// the bbox, return desired camera world matrix
fn fit_camera_view(
  proj: &PerspectiveProjection<f32>,
  camera_world: Mat4<f32>,
  target_world_aabb: Box3<f32>,
) -> Mat4<f32> {
  assert!(!target_world_aabb.is_empty());

  let padding_ratio = 0.1;
  let target_center = target_world_aabb.center();
  let object_radius = target_world_aabb.min.distance(target_center);

  //   if we not even have one box, only do look at
  let desired_camera_center = if object_radius == 0. {
    camera_world.position()
  } else {
    // todo also check horizon fov
    let half_fov = proj.fov.to_rad() / 2.;
    let canvas_half_size = half_fov.tan(); // todo consider near far limit
    let padded_canvas_half_size = canvas_half_size * (1.0 - padding_ratio);
    let desired_half_fov = padded_canvas_half_size.atan();
    let desired_distance = object_radius / desired_half_fov.sin();

    let look_at_dir_rev = (camera_world.position() - target_center).normalize();
    look_at_dir_rev * desired_distance + target_center
  };

  Mat4::lookat(desired_camera_center, target_center, Vec3::new(0., 1., 0.))
}
