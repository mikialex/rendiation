use crate::*;

#[derive(Default)]
pub struct InspectedContent {
  contents: FastHashMap<ShareKey, (String, SharedContentState)>,
  shared_stack: Vec<ShareKey>,
  root: SharedContentState,
}

#[derive(Default)]
struct SharedContentState {
  content: Vec<String>,
  memory_used: u64,
  device_memory_used: u64,
}

impl SharedContentState {
  fn clear(&mut self) {
    self.content.clear();
    self.memory_used = 0;
    self.device_memory_used = 0;
  }
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
        ui.heading("Summary:");

        let all_memory =
          self.root.memory_used + self.contents.values().map(|v| v.1.memory_used).sum::<u64>();
        let all_device_memory = self.root.device_memory_used
          + self
            .contents
            .values()
            .map(|v| v.1.device_memory_used)
            .sum::<u64>();

        ui.label(format!(
          "Total memory used: {}",
          self.format_readable_data_size(all_memory)
        ));
        ui.label(format!(
          "Total device memory used: {}",
          self.format_readable_data_size(all_device_memory)
        ));

        ui.heading("Root scope:");

        if self.root.content.is_empty() {
          ui.label("nothing to show");
        }

        for content in &self.root.content {
          ui.label(content);
        }

        ui.heading("Shared scopes:");

        if self.root.content.is_empty() {
          ui.label("nothing to show");
        }

        for (unique_k, (k, content)) in &self.contents {
          egui::CollapsingHeader::new(disqualified::ShortName(k).to_string())
            .id_salt(unique_k)
            .show(ui, |ui| {
              for content in content.content.iter() {
                ui.label(content);
              }
              if content.content.is_empty() {
                ui.label("nothing to show");
              }
            });
        }
      });
  }

  fn current_content(&mut self) -> &mut SharedContentState {
    if let Some(top) = self.shared_stack.last() {
      &mut self.contents.get_mut(top).unwrap().1
    } else {
      &mut self.root
    }
  }
}

impl Inspector for InspectedContent {
  fn label(&mut self, label: &str) {
    self.current_content().content.push(label.to_string());
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

  fn label_memory_usage(&mut self, label: &str, bytes: usize) {
    let readable = self.format_readable_data_size(bytes as u64);
    self.label(format!("\"{}\" mem used: {}", label, readable).as_str());
    self.current_content().memory_used += bytes as u64;
  }
  fn label_device_memory_usage(&mut self, label: &str, bytes: u64) {
    let readable = self.format_readable_data_size(bytes);
    self.label(format!("\"{}\" gpu mem used: {}", label, readable).as_str());
    self.current_content().device_memory_used += bytes;
  }
}
