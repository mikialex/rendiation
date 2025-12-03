use crate::*;

#[derive(Default)]
pub struct InspectedContent {
  contents: FastHashMap<ShareKey, (String, Vec<String>)>,
  shared_stack: Vec<ShareKey>,
  root: Vec<String>,
}

impl InspectedContent {
  pub fn clear(&mut self) {
    // keep the shared content;
    self.root.clear();
  }

  pub fn draw(&self, egui_ctx: &mut egui::Context) {
    egui::Window::new("System Inspection")
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if self.contents.is_empty() && self.root.is_empty() {
          ui.label("nothing to show");
          return;
        }

        ui.heading("Root scope:");

        for content in &self.root {
          ui.label(content);
        }

        ui.separator();

        ui.heading("Shared scopes:");

        for (unique_k, (k, content)) in &self.contents {
          egui::CollapsingHeader::new(disqualified::ShortName(k).to_string())
            .id_salt(unique_k)
            .show(ui, |ui| {
              for content in content {
                ui.label(content);
              }
            });
        }
      });
  }
}

impl Inspector for InspectedContent {
  fn label(&mut self, label: &str) {
    if let Some(top) = self.shared_stack.last() {
      let content = self.contents.get_mut(top).unwrap();
      content.1.push(label.to_string());
    } else {
      self.root.push(label.to_string());
    }
  }

  fn enter_shared_ctx(&mut self, key: &ShareKey, label: &str) {
    let (k, content) = self.contents.entry(*key).or_default();
    *k = label.to_string();
    content.clear();
    self.shared_stack.push(*key);
  }

  fn leave_shared_ctx(&mut self, key: &ShareKey) {
    let top = self.shared_stack.pop();
    assert_eq!(top, Some(*key));
  }

  fn drop_shared_ctx(&mut self, key: &ShareKey) {
    self.contents.remove(key);
  }
}
