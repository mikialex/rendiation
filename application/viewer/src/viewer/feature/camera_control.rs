use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};

use crate::*;

#[derive(Default)]
pub struct SceneOrbitCameraControl {
  pub controller: ControllerWinitAdapter<OrbitController>,
}

impl Widget for SceneOrbitCameraControl {
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx!(cx, event, PlatformEventInput);

    let bound = InputBound {
      origin: Vec2::zero(),
      size: event.window_state.size.into(),
    };

    for e in &event.accumulate_events {
      self.controller.event(e, bound)
    }

    access_cx!(cx, scene_cx, Viewer3dSceneCtx);

    let control_node = global_entity_component_of::<SceneCameraNode>()
      .read()
      .get(scene_cx.main_camera)
      .unwrap();
    let node_local_mat = global_entity_component_of::<SceneNodeLocalMatrixComponent>().write();

    self.controller.update(&mut ControlleeWrapper {
      controllee: todo!(),
      writer: node_local_mat,
    });
  }

  fn update_view(&mut self, _: &mut DynCx) {}
  fn clean_up(&mut self, _: &mut DynCx) {}
}

struct ControlleeWrapper {
  controllee: EntityHandle<SceneNodeEntity>,
  writer: ComponentWriteView<SceneNodeLocalMatrixComponent>,
}

impl Transformed3DControllee for ControlleeWrapper {
  fn get_matrix(&self) -> Mat4<f32> {
    self.writer.read(self.controllee).unwrap()
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.writer.write(self.controllee, m)
  }
}
