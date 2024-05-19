mod camera_control;
pub use camera_control::*;
mod fit_camera_view;
pub use fit_camera_view::*;

use crate::*;

pub fn core_viewer_features<V: Widget + 'static>(
  content_logic: impl Fn(&mut StateCx) -> V + 'static,
) -> impl Fn(&mut StateCx) -> Box<dyn Widget> {
  move |cx| {
    let gizmo = StateCxCreateOnce::new(|cx| {
      state_mut_access!(cx, scene_cx, Scene3dWriter);
      gizmo(scene_cx)
    });
    Box::new(
      WidgetGroup::default()
        .with_child(Viewer3dSceneCtxWriterWidget(gizmo))
        .with_child(SceneOrbitCameraControl::default())
        .with_child(content_logic(cx)),
    )
  }
}
