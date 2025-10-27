use crate::*;

/// currently, we only assume there is only one viewport
pub fn sync_camera_view(cx: &mut ViewerCx) {
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let viewport = &cx.viewer.scene.viewports[0];
    if cx.input.state_delta.size_change {
      writer
        .camera_writer
        .mutate_component_data::<SceneCameraPerspective>(viewport.camera, |p| {
          if let Some(p) = p.as_mut() {
            p.resize(cx.input.window_state.physical_size)
          }
        });
    }
  }
}
