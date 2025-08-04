use crate::*;

pub fn sync_camera_view(cx: &mut ViewerCx) {
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if cx.input.state_delta.size_change {
      writer
        .camera_writer
        .mutate_component_data::<SceneCameraPerspective>(cx.viewer.scene.main_camera, |p| {
          if let Some(p) = p.as_mut() {
            p.resize(cx.input.window_state.physical_size)
          }
        });
    }
  }
}
