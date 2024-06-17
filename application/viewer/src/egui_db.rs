use egui_extras::TableBuilder;

use crate::*;

fn egui_db_gui(ui: &mut egui::Context) {
  egui::Window::new("Viewer")
    .vscroll(true)
    .default_open(true)
    .min_width(500.0)
    .min_height(400.0)
    .default_width(800.0)
    .resizable(true)
    .movable(true)
    .anchor(egui::Align2::LEFT_TOP, [3.0, 3.0])
    .show(ui, |ui| {
      // let table = TableBuilder::new(ui);
    });
}
