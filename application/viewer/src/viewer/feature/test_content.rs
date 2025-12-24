use crate::*;

pub fn use_test_content_panel(cx: &mut ViewerCx) {
  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("test-content").or_insert(false);

    egui::Window::new("Test contents")
      .open(opened)
      .default_size((200., 200.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if ui.button("load many cubes").clicked() {
          load_stress_test(&mut SceneWriter::from_global(cx.viewer.content.scene))
        }

        if ui.button("test clipping").clicked() {
          test_clipping_data(cx.viewer.content.scene)
        }
      });
  }
}
