use rendiation_animation::*;
use winit::event::ElementState;

use crate::*;

pub struct CameraMoveAction {
  pub position: Vec3<f64>,
  pub look_at: Vec3<f64>,
}

pub fn use_smooth_camera_motion(cx: &mut ViewerCx, f: impl FnOnce(&mut ViewerCx)) {
  f(cx);

  let config = SpringConfig {
    frequency: 3.,
    damping: 1.0,
    initial_response: 0.9,
  };

  let (cx, target_position) = cx.use_plain_state_init(|_| Vec3::splat(3.));
  let (cx, springed_position) =
    cx.use_plain_state_init(|_| SpringSystem::new(config, *target_position, Vec3::zero()));

  let (cx, target_target) = cx.use_plain_state_init(|_| Vec3::splat(0.));
  let (cx, springed_target) =
    cx.use_plain_state_init(|_| SpringSystem::new(config, *target_target, Vec3::zero()));

  if let Some(CameraMoveAction { position, look_at }) = cx.dyn_cx.message.take::<CameraMoveAction>()
  {
    *target_position = position;
    *target_target = look_at;
  }

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut cx.stage {
    let time_delta_seconds = cx.time_delta_seconds as f64;
    let position = springed_position.step_clamped(time_delta_seconds, *target_position);
    let look_at = springed_target.step_clamped(time_delta_seconds, *target_target);

    let mat = Mat4::lookat(position, look_at, Vec3::new(0., 1., 0.));
    let node = cx.viewer.scene.camera_node;
    writer.set_local_matrix(node, mat);
  }
}

pub fn use_fit_camera_view(cx: &mut ViewerCx) {
  let sm_world_bounding = cx
    .use_shared_dual_query_view(SceneModelWorldBounding)
    .use_assure_result(cx);

  let world_mat = use_global_node_world_mat_view(cx).use_assure_result(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if let Some(ElementState::Pressed) = cx
      .input
      .state_delta
      .key_state_changes
      .get(&winit::keyboard::KeyCode::KeyF)
    {
      let world_mat = world_mat.expect_resolve_stage().mark_entity_type();
      let sm_world_bounding = sm_world_bounding.expect_resolve_stage().mark_entity_type();
      if let Some(action) =
        fit_camera_view_for_viewer(&cx.viewer.scene, world_mat, sm_world_bounding)
      {
        cx.dyn_cx.message.put(action);
      }
    }
  }
}

fn fit_camera_view_for_viewer(
  scene_info: &Viewer3dContent,
  world_mat: impl Query<Key = EntityHandle<SceneNodeEntity>, Value = Mat4<f64>>,
  sm_world_bounding: impl Query<Key = EntityHandle<SceneModelEntity>, Value = Box3<f64>>,
) -> Option<CameraMoveAction> {
  if let Some(selected) = &scene_info.selected_target {
    let camera_world = world_mat.access(&scene_info.camera_node).unwrap();
    let camera_reader = global_entity_component_of::<SceneCameraPerspective>().read();

    if let Some(target_world_aabb) = sm_world_bounding.access(selected) {
      let proj = camera_reader.get(scene_info.main_camera).unwrap().unwrap();

      fit_camera_view(&proj, camera_world, target_world_aabb).into()
    } else {
      log::warn!(
        "fit_camera unable to access selected target bounding box, it's a bug or missing implementation"
      );
      None
    }
  } else {
    None
  }
}

/// Target_world_aabb should not empty. If the target is unbound, the center point of the passed box
/// should be logical target center. Return desired camera world matrix
fn fit_camera_view(
  proj: &PerspectiveProjection<f32>,
  camera_world: Mat4<f64>,
  target_world_aabb: Box3<f64>,
) -> CameraMoveAction {
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
    let desired_distance = object_radius / desired_half_fov.sin() as f64;

    let look_at_dir_rev = (camera_world.position() - target_center).normalize();
    look_at_dir_rev * desired_distance + target_center
  };

  CameraMoveAction {
    position: desired_camera_center,
    look_at: target_center,
  }
}
