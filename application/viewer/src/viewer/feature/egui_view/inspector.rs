use crate::*;

#[derive(Default)]
pub struct InspectedContent {
  contents: Vec<String>,
}

impl InspectedContent {
  pub fn draw(&self, egui_ctx: &mut egui::Context) {
    egui::Window::new("System Inspection").show(egui_ctx, |ui| {
      if self.contents.is_empty() {
        ui.label("nothing to show");
      }

      for content in &self.contents {
        ui.label(content);
      }
    });
  }
}

impl Inspector for InspectedContent {
  fn label(&mut self, label: &str) {
    self.contents.push(label.to_string());
  }
}
