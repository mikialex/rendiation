use crate::*;

pub fn sync_camera_view(cx: &mut ViewerCx) {
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    for viewport in &mut cx.viewer.content.viewports {
      if viewport.viewport.z > 0. && viewport.viewport.w > 0. {
        writer
          .camera_writer
          .mutate_component_data::<SceneCameraPerspective>(viewport.camera, |p| {
            if let Some(p) = p.as_mut() {
              p.resize((viewport.viewport.z, viewport.viewport.w))
            }
          });

        writer
          .camera_writer
          .mutate_component_data::<SceneCameraOrthographic>(viewport.camera, |p| {
            if let Some(p) = p.as_mut() {
              let ratio = viewport.viewport.z / viewport.viewport.w;

              let mut new_size = p.size();
              new_size.x = new_size.y * ratio;
              let scale = new_size / p.size();
              p.scale_from_center(scale)
            }
          });
      }
    }
  }
}
