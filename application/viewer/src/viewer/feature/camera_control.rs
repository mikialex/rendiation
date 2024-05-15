use rendiation_controller::{ControllerWinitAdapter, InputBound, OrbitController};

use crate::*;

pub struct SceneOrbitCameraControl {
  pub controller: ControllerWinitAdapter<OrbitController>,
}

impl Widget for SceneOrbitCameraControl {
  fn update_state(&mut self, cx: &mut StateCx) {
    state_access!(cx, event, PlatformEventInput);

    let bound = InputBound {
      origin: todo!(),
      size: todo!(),
    };

    for e in &event.accumulate_events {
      // self.controller.event(e, bound)
    }

    // todo update camera state
  }

  fn update_view(&mut self, _: &mut StateCx) {}
  fn clean_up(&mut self, _: &mut StateCx) {}
}

// struct ControlleeWrapper<'a> {
//   controllee: &'a SceneNode,
// }

// impl<'a> Transformed3DControllee for ControlleeWrapper<'a> {
//   fn get_matrix(&self) -> Mat4<f32> {
//     self.controllee.get_local_matrix()
//   }

//   fn set_matrix(&mut self, m: Mat4<f32>) {
//     self.controllee.set_local_matrix(m)
//   }
// }
