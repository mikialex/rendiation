use rendiation_controller::*;

use crate::*;

#[derive(Default)]
pub struct SceneCameraControl {
  pub controller: OrbitController,
  pub winit_state: OrbitWinitWindowState,
}

pub struct CameraControlBlocked;

pub fn use_camera_control(cx: &mut ViewerCx) {
  let (cx, controller) = cx.use_plain_state::<SceneCameraControl>();

  match &mut cx.stage {
    ViewerCxStage::EventHandling { input, .. } => {
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
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      let camera = cx.viewer.scene.camera_node;
      if let Some(mat) = controller.controller.update() {
        writer.set_local_matrix(camera, mat)
      }
    }
    _ => {}
  }
}
