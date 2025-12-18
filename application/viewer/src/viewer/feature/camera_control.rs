use rendiation_controller::*;

use crate::{viewer::use_scene_reader, *};

#[derive(Default)]
pub struct ViewerCameraControl {
  pub controller: OrbitController,
  pub winit_state: OrbitWinitWindowState,
  pub have_synced_for_viewer_init_camera_state: bool,
}

pub struct CameraControlBlocked;

pub fn use_camera_control(cx: &mut ViewerCx, camera_with_viewports: &CameraViewportAccess) {
  let (cx, controller) = cx.use_plain_state::<ViewerCameraControl>();

  // if inner logic want change camera, then we adapt to  it
  if let Some(CameraAction {
    position, look_at, ..
  }) = cx.dyn_cx.message.get::<CameraAction>()
  {
    controller
      .controller
      .update_target_and_position(*look_at, *position);
  }

  let reader = use_scene_reader(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    if !controller.have_synced_for_viewer_init_camera_state {
      let camera_local = reader
        .unwrap()
        .node_reader
        .read::<SceneNodeLocalMatrixComponent>(camera_with_viewports.camera_node);
      let lookat_target_init = camera_local * Vec3::new(0., 0., -1.);
      controller
        .controller
        .update_target_and_position(lookat_target_init, camera_local.position());
      controller.have_synced_for_viewer_init_camera_state = true;
    }

    if cx.dyn_cx.message.take::<CameraControlBlocked>().is_some() {
      controller.winit_state.pause();
      return;
    }

    let mouse_position = &cx.input.window_state.mouse_position; // todo, use surface relative position
    let viewports = camera_with_viewports
      .viewports_index
      .iter()
      .map(|(index, _)| &cx.viewer.content.viewports[*index]);
    if let Some((viewport, _)) = find_top_hit(viewports, *mouse_position) {
      let bound = viewport_to_input_bound(viewport.viewport);

      for e in &cx.input.accumulate_events {
        controller
          .controller
          .event(&mut controller.winit_state, e, bound);
      }

      if let Some((eye, target)) = controller.controller.update() {
        cx.dyn_cx.message.put(CameraAction {
          position: eye,
          look_at: target,
          orth_scale: None,
        });
      }
    } else {
      controller.winit_state.pause();
    }
  }
}
