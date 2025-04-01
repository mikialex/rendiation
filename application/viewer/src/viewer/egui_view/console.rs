use std::sync::atomic::AtomicU16;

use egui::*;

pub struct Console {
  pub buffer: String,
  id: egui::Id,
}

static INSTANCE_COUNT: AtomicU16 = AtomicU16::new(0);
impl Console {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      buffer: Default::default(),
      id: Id::new(format!(
        "console_text_{}",
        INSTANCE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
      )),
    }
  }

  pub fn current_input(&self) -> Option<&str> {
    if self.buffer.ends_with("\n") || self.buffer.ends_with("\r\n") {
      return None;
    }
    self.buffer.lines().last()
  }

  pub fn writeln(&mut self, data: impl AsRef<str>) {
    let data = data.as_ref();
    println!("{}", data);
    if let Some(current) = self.current_input() {
      let current = current.to_owned();
      for _ in 0..current.chars().count() {
        self.buffer.pop();
      }

      self.buffer.push_str(data);
      self.buffer.push('\n');

      self.buffer.push_str(current.as_str());
    } else {
      self.buffer.push_str(data);
      self.buffer.push('\n');
    }
  }

  #[allow(clippy::single_match)]
  pub fn ui(&mut self, ui: &mut Ui) -> Option<String> {
    let mut console_response = None;
    //  handle keyboard events if we have focus
    if ui.ctx().memory(|mem| mem.has_focus(self.id)) {
      ui.ctx().input(|input| {
        for event in &input.events {
          if let Event::Key {
            key, pressed: true, ..
          } = event
          {
            match key {
              Key::Enter => {
                if let Some(input) = self.current_input() {
                  let command = input.to_owned();
                  console_response = Some(command.clone());
                }
              }
              _ => {}
            }
          }
        }
      });
    };

    egui::ScrollArea::both().show(ui, |ui| {
      ui.add_sized(ui.available_size(), |ui: &mut Ui| {
        let widget = egui::TextEdit::multiline(&mut self.buffer)
          .font(egui::TextStyle::Monospace)
          .frame(false)
          .code_editor()
          .lock_focus(true)
          .desired_width(f32::INFINITY)
          .id(self.id);
        let output = widget.show(ui);

        let new_cursor =
          egui::text::CCursorRange::one(egui::text::CCursor::new(self.buffer.chars().count()));
        let text_edit_id = output.response.id;

        if let Some(mut state) = TextEdit::load_state(ui.ctx(), text_edit_id) {
          state.cursor.set_char_range(Some(new_cursor));
          state.store(ui.ctx(), text_edit_id);
        }
        ui.scroll_to_cursor(Some(Align::BOTTOM));

        output.response
      });
    });

    console_response
  }
}
