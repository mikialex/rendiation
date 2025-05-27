use rendiation_controller::*;

use crate::*;

#[derive(Default)]
pub struct ViewerCameraControl {
  pub controller: OrbitController,
  pub winit_state: OrbitWinitWindowState,
  pub have_synced_for_viewer_init_camera_state: bool,
}

pub struct CameraControlBlocked;

pub fn use_camera_control(cx: &mut ViewerCx) {
  let (cx, controller) = cx.use_plain_state::<ViewerCameraControl>();

  // if inner logic want change camera, then we adapt to  it
  if let Some(CameraMoveAction { position, look_at }) = cx.dyn_cx.message.get::<CameraMoveAction>()
  {
    controller
      .controller
      .update_target_and_position(*look_at, *position);
  }

  if let ViewerCxStage::EventHandling { input, reader, .. } = &mut cx.stage {
    if !controller.have_synced_for_viewer_init_camera_state {
      let camera_local = reader
        .node_reader
        .read::<SceneNodeLocalMatrixComponent>(cx.viewer.scene.camera_node);
      let lookat_target_init = camera_local * Vec3::new(0., 0., -1.);
      controller
        .controller
        .update_target_and_position(lookat_target_init, camera_local.position());
      controller.have_synced_for_viewer_init_camera_state = true;
    }

    let pause = cx.dyn_cx.message.take::<CameraControlBlocked>().is_some();

    let bound = InputBound {
      origin: Vec2::zero(),
      size: input.window_state.physical_size.into(),
    };

    for e in &input.accumulate_events {
      controller
        .controller
        .event(&mut controller.winit_state, e, bound, pause);
    }

    if let Some((eye, target)) = controller.controller.update() {
      cx.dyn_cx.message.put(CameraMoveAction {
        position: eye,
        look_at: target,
      });
    }
  }
}
