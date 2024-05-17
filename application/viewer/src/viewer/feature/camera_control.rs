use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};

use crate::*;

pub struct SceneOrbitCameraControl {
  pub controller: ControllerWinitAdapter<OrbitController>,
}

impl Widget for SceneOrbitCameraControl {
  fn update_state(&mut self, cx: &mut StateCx) {
    state_access!(cx, event, PlatformEventInput);

    let bound = InputBound {
      origin: Vec2::zero(),
      size: event.window_state.size.into(),
    };

    for e in &event.accumulate_events {
      self.controller.event(e, bound)
    }

    state_access!(cx, scene_cx, Viewer3dSceneCtx);

    let control_node = global_entity_component_of::<SceneCameraNode>()
      .read()
      .get(scene_cx.main_camera.alloc_idx())
      .unwrap();
    let node_local_mat = global_entity_component_of::<SceneNodeLocalMatrixComponent>().write();

    self.controller.update(&mut ControlleeWrapper {
      controllee: todo!(),
      writer: node_local_mat,
    });
  }

  fn update_view(&mut self, _: &mut StateCx) {}
  fn clean_up(&mut self, _: &mut StateCx) {}
}

struct ControlleeWrapper {
  controllee: EntityHandle<SceneNodeLocalMatrixComponent>,
  writer: ComponentWriteView<SceneNodeLocalMatrixComponent>,
}

impl Transformed3DControllee for ControlleeWrapper {
  fn get_matrix(&self) -> Mat4<f32> {
    self.writer.read(self.controllee.alloc_idx().index)
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.writer.write(self.controllee.alloc_idx().index, m)
  }
}
