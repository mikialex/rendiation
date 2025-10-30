use crate::*;

/// use this to test viewport
pub fn sync_camera_view(cx: &mut ViewerCx) {
  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let viewport = &mut cx.viewer.scene.viewports[0];

    let size = cx.input.window_state.physical_size;
    viewport.viewport = Vec4::new(0., 0., size.0 / 2., size.1);

    if cx.input.state_delta.size_change {
      writer
        .camera_writer
        .mutate_component_data::<SceneCameraPerspective>(viewport.camera, |p| {
          if let Some(p) = p.as_mut() {
            p.resize((
              cx.input.window_state.physical_size.0 / 2.,
              cx.input.window_state.physical_size.1,
            ));
          }
        });
    }
  }
}

// /// currently, we only assume there is only one viewport
// pub fn sync_camera_view(cx: &mut ViewerCx) {
//   if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
//     let viewport = &mut cx.viewer.scene.viewports[0];

//     let size = cx.input.window_state.physical_size;
//     viewport.viewport = Vec4::new(0., 0., size.0, size.1);

//     if cx.input.state_delta.size_change {
//       writer
//         .camera_writer
//         .mutate_component_data::<SceneCameraPerspective>(viewport.camera, |p| {
//           if let Some(p) = p.as_mut() {
//             p.resize(cx.input.window_state.physical_size)
//           }
//         });
//     }
//   }
// }
