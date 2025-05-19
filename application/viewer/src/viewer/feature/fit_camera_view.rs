use crate::*;

pub fn use_fit_camera_view(cx: &mut ViewerCx) {
  cx.use_state_init(|cx| {
    cx.terminal
      .register_sync_command("fit-camera-view", |ctx, _parameters| {
        let derived = ctx.derive.poll_update();
        if let Some(selected) = &ctx.scene.selected_target {
          let camera_world = derived.world_mat.access(&ctx.scene.camera_node).unwrap();
          let camera_reader = global_entity_component_of::<SceneCameraPerspective>().read();

          let target_world_aabb = derived.sm_world_bounding.access(selected).unwrap();
          let proj = camera_reader.get(ctx.scene.main_camera).unwrap().unwrap();

          let camera_world = fit_camera_view(&proj, camera_world, target_world_aabb);
          // todo fix camera has parent mat
          global_entity_component_of::<SceneNodeLocalMatrixComponent>()
            .write()
            .write(ctx.scene.camera_node, camera_world);
        }
      });
    FitCameraViewForViewer
  });
}

struct FitCameraViewForViewer;
impl CanCleanUpFrom<ViewerDropCx<'_>> for FitCameraViewForViewer {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command("fit-camera-view");
  }
}

/// Target_world_aabb should not empty. If the target is unbound, should give a box that the center point
/// is the logical target center. Return desired camera world matrix
fn fit_camera_view(
  proj: &PerspectiveProjection<f32>,
  camera_world: Mat4<f32>,
  target_world_aabb: Box3<f32>,
) -> Mat4<f32> {
  assert!(!target_world_aabb.is_empty());

  let padding_ratio = 0.1;
  let target_center = target_world_aabb.center();
  let object_radius = target_world_aabb.min.distance_to(target_center);

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
