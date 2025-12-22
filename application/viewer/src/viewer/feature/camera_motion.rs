use rendiation_animation::*;
use winit::event::ElementState;

use crate::*;

pub struct CameraAction {
  pub position: Vec3<f64>,
  pub look_at: Vec3<f64>,
  pub orth_scale: Option<f64>,
}

pub fn use_smooth_camera_motion(
  cx: &mut ViewerCx,
  camera_node: EntityHandle<SceneNodeEntity>,
  camera: EntityHandle<SceneCameraEntity>,
  f: impl FnOnce(&mut ViewerCx),
) {
  f(cx);

  let config = SpringConfig {
    frequency: 3.,
    damping: 1.0,
    initial_response: 0.9,
  };

  // todo, fix init camera target position and target not match the camera current position and target.
  let (cx, target_position) = cx.use_plain_state_init(|_| Vec3::splat(3.));
  let (cx, springed_position) =
    cx.use_plain_state_init(|_| SpringSystem::new(config, *target_position, Vec3::zero()));

  let (cx, target_target) = cx.use_plain_state_init(|_| Vec3::splat(0.));
  let (cx, springed_target) =
    cx.use_plain_state_init(|_| SpringSystem::new(config, *target_target, Vec3::zero()));

  let config = SpringConfig {
    frequency: 3.,
    damping: 1.0,
    initial_response: -10.,
  };

  let (cx, projection_target) = cx.use_plain_state_init(|_| Mat4::identity());
  let (cx, springed_projection_target) =
    cx.use_plain_state_init(|_| SpringSystem::new(config, *projection_target, Mat4::zero()));

  let (cx, orth_scale_to_apply) = cx.use_plain_state_init(|_| None);

  if let Some(CameraAction {
    position,
    look_at,
    orth_scale,
  }) = cx.dyn_cx.message.take::<CameraAction>()
  {
    *target_position = position;
    *target_target = look_at;
    *orth_scale_to_apply = orth_scale;
  }

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut cx.stage {
    let time_delta_seconds = cx.time_delta_seconds as f64;
    let position = springed_position.step_clamped(time_delta_seconds, *target_position);
    let look_at = springed_target.step_clamped(time_delta_seconds, *target_target);

    let mat = Mat4::lookat(position, look_at, Vec3::new(0., 1., 0.));
    writer.set_local_matrix(camera_node, mat);

    if let Some(orth_scale) = orth_scale_to_apply.take() {
      if let Some(mut orth) = writer.camera_writer.read::<SceneCameraOrthographic>(camera) {
        if orth_scale > 0. {
          orth.scale_from_center(Vec2::splat(orth_scale as f32));
          writer
            .camera_writer
            .write::<SceneCameraOrthographic>(camera, Some(orth));
        } else {
          log::warn!("orth_scale_to_apply should not be negative, {}", orth_scale);
        }
      }
    }

    if let Some(p) = writer.camera_writer.read::<SceneCameraOrthographic>(camera) {
      *projection_target = p.compute_projection_mat(&OpenGLxNDC).into_f64();
    }
    if let Some(p) = writer.camera_writer.read::<SceneCameraPerspective>(camera) {
      *projection_target = p.compute_projection_mat(&OpenGLxNDC).into_f64();
    }
    let projection = springed_projection_target
      .step_clamped(time_delta_seconds, *projection_target)
      .into_f32();
    writer
      .camera_writer
      .write::<SceneCameraProjectionCustomOverride>(camera, Some(projection));
  }
}

pub fn use_fit_camera_view(
  cx: &mut ViewerCx,
  camera: EntityHandle<SceneCameraEntity>,
  camera_node: EntityHandle<SceneNodeEntity>,
) {
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
      if let Some(action) = fit_camera_view_for_viewer(
        camera,
        camera_node,
        cx.viewer.content.selected_model,
        world_mat,
        sm_world_bounding,
      ) {
        cx.dyn_cx.message.put(action);
      }
    }
  }
}

fn fit_camera_view_for_viewer(
  camera: EntityHandle<SceneCameraEntity>,
  camera_node: EntityHandle<SceneNodeEntity>,
  selected_sm: Option<EntityHandle<SceneModelEntity>>,
  world_mat: impl Query<Key = EntityHandle<SceneNodeEntity>, Value = Mat4<f64>>,
  sm_world_bounding: impl Query<Key = EntityHandle<SceneModelEntity>, Value = Box3<f64>>,
) -> Option<CameraAction> {
  if let Some(selected) = &selected_sm {
    let camera_world = world_mat.access(&camera_node).unwrap();

    if let Some(target_world_aabb) = sm_world_bounding.access(selected) {
      if let Some(camera_proj) = read_common_proj_from_db(camera) {
        return fit_camera_view(camera_proj, camera_world, target_world_aabb);
      } else {
        log::warn!("fit_camera failed to get common proj for camera");
      }
    } else {
      log::warn!(
        "fit_camera unable to access selected target bounding box, it's a bug or missing implementation"
      );
    }
  }
  None
}

/// Target_world_aabb should not empty. If the target is unbound, the center point of the passed box
/// should be logical target center. Return desired camera world matrix
fn fit_camera_view(
  proj: CommonProjection,
  camera_world: Mat4<f64>,
  target_world_aabb: Box3<f64>,
) -> Option<CameraAction> {
  if target_world_aabb.is_empty() {
    log::warn!("target world aabb is empty, fit_camera_view skipped");
  }

  let padding_ratio = 0.1;
  let target_center = target_world_aabb.center();
  let object_radius = target_world_aabb.min.distance_to(target_center);

  match proj {
    CommonProjection::Perspective(proj) => {
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

      CameraAction {
        position: desired_camera_center,
        look_at: target_center,
        orth_scale: None,
      }
      .into()
    }
    // todo, in this case not change rotation is a better behavior
    CommonProjection::Orth(proj) => {
      let size = proj.size();
      let size = size.x.min(size.y);

      let target_size = object_radius * 2.0 * (1.0 + padding_ratio as f64);
      let scale = target_size / size as f64;

      // todo, currently we assume the orth is centered
      // let orth_center = Vec3::new(
      //   proj.right - proj.left / 2.0,
      //   proj.top - proj.bottom / 2.0,
      //   0.0,
      // )
      // .into_f64()
      // .reverse();

      CameraAction {
        // position: camera_world * orth_center,
        position: camera_world.position(),
        look_at: target_center,
        orth_scale: Some(scale),
      }
      .into()
    }
  }
}
