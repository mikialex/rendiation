use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};

use crate::*;

#[derive(Default)]
pub struct SceneOrbitCameraControl {
  pub controller: ControllerWinitAdapter<OrbitController>,
}

pub struct CameraControlBlocked;

pub fn use_camera_orbit_control(cx: &mut ViewerCx) {
  let (cx, controller) = cx.use_plain_state::<ControllerWinitAdapter<OrbitController>>();

  match &mut cx.stage {
    ViewerCxStage::EventHandling { input, .. } => {
      let pause = cx.dyn_cx.message.take::<CameraControlBlocked>().is_some();

      let bound = InputBound {
        origin: Vec2::zero(),
        size: input.window_state.physical_size.into(),
      };

      for e in &input.accumulate_events {
        controller.event(e, bound, pause);
      }
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      let controllee = cx.viewer.scene.camera_node;
      controller.update(&mut ControlleeWrapper { controllee, writer });
    }
    _ => {}
  }
}

struct ControlleeWrapper<'a> {
  controllee: EntityHandle<SceneNodeEntity>,
  writer: &'a mut SceneWriter,
}

impl Transformed3DControllee for ControlleeWrapper<'_> {
  fn get_matrix(&self) -> Mat4<f32> {
    self.writer.get_local_mat(self.controllee).unwrap()
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.writer.set_local_matrix(self.controllee, m)
  }
}
