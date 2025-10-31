use crate::*;

pub fn sync_camera_view(cx: &mut ViewerCx) {
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    for viewport in &mut cx.viewer.scene.viewports {
      writer
        .camera_writer
        .mutate_component_data::<SceneCameraPerspective>(viewport.camera, |p| {
          if let Some(p) = p.as_mut() {
            p.resize((viewport.viewport.z, viewport.viewport.w))
          }
        });
    }
  }
}
