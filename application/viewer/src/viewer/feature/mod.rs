mod camera_control;
pub use camera_control::*;
mod gizmo_bridge;
pub use gizmo_bridge::*;
mod fit_camera_view;
pub use fit_camera_view::*;
mod pick_scene;
pub use pick_scene::*;
mod camera_helper;
pub use camera_helper::*;

use crate::*;

pub fn core_viewer_features<V: Widget + 'static>(
  content_logic: impl Fn(&mut DynCx) -> V + 'static,
) -> impl Fn(&mut DynCx) -> Box<dyn Widget> {
  move |cx| {
    Box::new(
      WidgetGroup::default()
        .with_child(StateCxCreateOnce::create_at_view(GizmoBridge::new))
        .with_child(SceneOrbitCameraControl::default())
        .with_child(PickScene {
          enable_hit_debug_log: false,
          use_gpu_pick: true,
          gpu_pick_future: Default::default(),
        })
        .with_child(content_logic(cx)),
    )
  }
}
