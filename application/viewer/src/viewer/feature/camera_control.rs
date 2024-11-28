use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};

use crate::*;

#[derive(Default)]
pub struct SceneOrbitCameraControl {
  pub controller: ControllerWinitAdapter<OrbitController>,
}

pub struct CameraControlBlocked;

impl Widget for SceneOrbitCameraControl {
  fn update_state(&mut self, cx: &mut DynCx) {
    if cx.message.take::<CameraControlBlocked>().is_some() {
      return;
    }

    access_cx!(cx, p, PlatformEventInput);

    let bound = InputBound {
      origin: Vec2::zero(),
      size: p.window_state.size.into(),
    };

    for e in &p.accumulate_events {
      self.controller.event(e, bound)
    }
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx!(cx, scene_cx, Viewer3dSceneCtx);
    let controllee = scene_cx.camera_node;
    access_cx_mut!(cx, writer, SceneWriter);

    self
      .controller
      .update(&mut ControlleeWrapper { controllee, writer });
  }
  fn clean_up(&mut self, _: &mut DynCx) {}
}

struct ControlleeWrapper<'a> {
  controllee: EntityHandle<SceneNodeEntity>,
  writer: &'a mut SceneWriter,
}

impl<'a> Transformed3DControllee for ControlleeWrapper<'a> {
  fn get_matrix(&self) -> Mat4<f32> {
    self.writer.get_local_mat(self.controllee).unwrap()
  }

  fn set_matrix(&mut self, m: Mat4<f32>) {
    self.writer.set_local_matrix(self.controllee, m)
  }
}
