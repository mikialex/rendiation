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
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, scene_cx, Viewer3dSceneCtx);
    let controllee = scene_cx.camera_node;
    access_cx_mut!(cx, writer, Scene3dWriter);

    self
      .controller
      .update(&mut ControlleeWrapper { controllee, writer });
  }
  fn clean_up(&mut self, _: &mut DynCx) {}
}

struct ControlleeWrapper<'a> {
  controllee: EntityHandle<SceneNodeEntity>,
  writer: &'a mut Scene3dWriter,
}

impl<'a> Transformed3DControllee for ControlleeWrapper<'a> {
  fn get_matrix(&self) -> Mat4<f32> {
    self.writer.get_local_mat(self.controllee).unwrap()
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.writer.set_local_matrix(self.controllee, m)
  }
}
