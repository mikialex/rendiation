mod camera_control;
pub use camera_control::*;
mod selection;
pub use selection::*;
mod selection_control;
pub use selection_control::*;
mod fit_camera_view;
pub use fit_camera_view::*;

use crate::*;

pub fn core_viewer_features<V: Widget>(
  content_logic: impl Fn(&mut StateCx) -> V + 'static,
) -> impl Fn(&mut StateCx) -> Box<dyn Widget> {
  move |cx| {
    Box::new(
      WidgetGroup::default()
        .with_child(gizmo(todo!()))
        .with_child(content_logic(cx)),
    )
  }
}
