use winit::event::ElementState;

use crate::*;

pub fn use_fit_camera_view(cx: &mut ViewerCx) {
  cx.use_state_init(|cx| {
    cx.terminal
      .register_sync_command("fit-camera-view", |ctx, _parameters| {
        let derived = ctx.derive.poll_update();

        if let Some((node, mat)) = fit_camera_view_for_viewer(ctx.scene, &derived) {
          global_entity_component_of::<SceneNodeLocalMatrixComponent>()
            .write()
            .write(mat, node);
        }
      });
    FitCameraViewForViewer
  });

  let (cx, mat_to_sync) =
    cx.use_plain_state::<Option<(Mat4<f32>, EntityHandle<SceneNodeEntity>)>>();

  if let ViewerCxStage::EventHandling { derived, input, .. } = &mut cx.stage {
    if let Some(ElementState::Pressed) = input
      .state_delta
      .key_state_changes
      .get(&winit::keyboard::KeyCode::KeyF)
    {
      *mat_to_sync = fit_camera_view_for_viewer(&cx.viewer.scene, derived);
    }
  }

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut cx.stage {
    if let Some((mat, node)) = mat_to_sync.take() {
      writer.set_local_matrix(node, mat);
    }
  }
}

fn fit_camera_view_for_viewer(
  scene_info: &Viewer3dSceneCtx,
  derived: &Viewer3dSceneDerive,
) -> Option<(Mat4<f32>, EntityHandle<SceneNodeEntity>)> {
  if let Some(selected) = &scene_info.selected_target {
    let camera_world = derived.world_mat.access(&scene_info.camera_node).unwrap();
    let camera_reader = global_entity_component_of::<SceneCameraPerspective>().read();

    let target_world_aabb = derived.sm_world_bounding.access(selected).unwrap();
    let proj = camera_reader.get(scene_info.main_camera).unwrap().unwrap();

    let camera_world = fit_camera_view(&proj, camera_world, target_world_aabb);
    // todo fix camera has parent mat
    (camera_world, scene_info.camera_node).into()
  } else {
    None
  }
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
