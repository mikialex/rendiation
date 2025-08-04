use crate::*;

pub fn use_sync_camera_view(cx: &mut ViewerCx) {
  let (cx, size) = cx.use_plain_state::<(f32, f32)>();
  let (cx, size_changed) = cx.use_plain_state::<bool>();

  if let ViewerCxStage::EventHandling { input, .. } = cx.stage {
    *size = input.window_state.physical_size;
    *size_changed = input.state_delta.size_change;
  }

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if *size_changed {
      writer
        .camera_writer
        .mutate_component_data::<SceneCameraPerspective>(cx.viewer.scene.main_camera, |p| {
          if let Some(p) = p.as_mut() {
            p.resize(*size)
          }
        });
    }
  }
}
