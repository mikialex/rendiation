use crate::*;

pub struct EditableText {
  text: Text,
  cursor: Option<Cursor>,
  on_change: Option<Box<dyn Fn(&mut String)>>,
}

use std::{
  ops::{Deref, DerefMut},
  time::Duration,
};
impl Deref for EditableText {
  type Target = Text;

  fn deref(&self) -> &Self::Target {
    &self.text
  }
}

impl DerefMut for EditableText {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.text
  }
}

impl EditableText {
  #[must_use]
  pub fn on_change(mut self, on_change: impl Fn(&mut String) + 'static) -> Self {
    self.on_change = Some(Box::new(on_change));
    self
  }

  // when model updated by user side
  // cursor position maybe overflow the text length
  // so we simply clamp it
  fn clamp_cursor_position(&mut self) {
    if let Some(cursor) = &mut self.cursor {
      cursor.set_index(cursor.get_index().clamp(0, self.text.content.get().len()));
    }
  }

  fn update_cursor_by_click(
    &mut self,
    position: UIPosition,
    fonts: &FontManager,
    texts: &mut TextCache,
  ) {
    let layout = self.text.get_text_layout(fonts, texts);
    let rect = layout
      .layout()
      .glyphs
      .iter()
      .map(|(_, _, rect)| rect)
      .enumerate()
      .find(|(_, rect)| {
        position.x >= rect.left_top[0]
          && position.x <= rect.right_bottom[0]
          && position.y >= rect.left_top[1]
          && position.y <= rect.right_bottom[1]
      });

    if let Some((index, rect)) = rect {
      let text_index = if position.x >= (rect.left_top[0] + rect.right_bottom[0]) / 2. {
        index + 1
      } else {
        index
      };

      self.cursor = Cursor::new(text_index).into()
    } else {
      self.cursor = None;
    }
  }

  fn insert_at_cursor(&mut self, c: char, model: &mut String) {
    if c.is_control() {
      return;
    }
    if let Some(cursor) = &mut self.cursor {
      let index = cursor.get_index();
      model.insert(index, c);

      self.text.content.set(model.clone());
      self.text.reset_text_layout_cache();
      cursor.notify_text_layout_changed();
      cursor.move_right();
    }
  }

  fn delete_at_cursor(&mut self, model: &mut String) {
    if let Some(cursor) = &mut self.cursor {
      if cursor.get_index() == 0 {
        // if cursor at first, cant delete
        return;
      }
      model.remove(cursor.get_index() - 1);

      self.text.content.set(model.clone());
      self.text.reset_text_layout_cache();
      cursor.notify_text_layout_changed();
      cursor.move_left();
    }
  }

  fn move_cursor(&mut self, dir: CursorMove) {
    if let Some(cursor) = &mut self.cursor {
      match dir {
        CursorMove::Left => {
          if cursor.get_index() != 0 {
            cursor.move_left();
          }
        }
        CursorMove::Right => {
          if cursor.get_index() != self.text.content.get().len() {
            cursor.move_right();
          }
        }
        CursorMove::Up => {} // todo
        CursorMove::Down => {}
      }
    }
  }

  fn handle_input(&mut self, key: winit::event::VirtualKeyCode, model: &mut String) {
    use winit::event::VirtualKeyCode::*;
    match key {
      Left => self.move_cursor(CursorMove::Left),
      Up => self.move_cursor(CursorMove::Up),
      Right => self.move_cursor(CursorMove::Right),
      Down => self.move_cursor(CursorMove::Down),
      Back => self.delete_at_cursor(model),
      _ => {}
    }
  }
}

impl Text {
  pub fn editable(self) -> EditableText {
    EditableText {
      text: self,
      cursor: None,
      on_change: None,
    }
  }
}

impl Component<String> for EditableText {
  fn event(&mut self, model: &mut String, ctx: &mut EventCtx) {
    self.text.event(model, ctx);

    use winit::event::*;

    let mut changed = false;

    match ctx.event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::KeyboardInput { input, .. } => {
          if let Some(virtual_keycode) = input.virtual_keycode {
            if input.state == ElementState::Pressed {
              self.handle_input(virtual_keycode, model);
              changed = true;
            }
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if let (MouseButton::Left, ElementState::Pressed) = (button, state) {
            self.update_cursor_by_click(ctx.states.mouse_position, ctx.fonts, ctx.texts)
          }
        }
        WindowEvent::ReceivedCharacter(char) => {
          self.insert_at_cursor(*char, model);
          changed = true;
        }
        _ => {}
      },
      _ => {}
    }

    if changed {
      if let Some(on_change) = &self.on_change {
        on_change(model);
      }
    }
  }

  fn update(&mut self, model: &String, ctx: &mut UpdateCtx) {
    self.text.content.set(model);
    self.clamp_cursor_position();
    self.text.update(model, ctx)
  }
}

fn blink_show(dur: Duration) -> bool {
  let time = dur.as_millis();
  time % 1000 > 500
}

impl Presentable for EditableText {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.text.render(builder);
    if let Some(cursor) = &mut self.cursor {
      if blink_show(cursor.get_last_update_timestamp().elapsed()) {
        return;
      }

      let layout = self.text.get_text_layout(builder.fonts, builder.texts);
      builder.present.primitives.push(Primitive::Quad((
        cursor.create_quad(layout),
        Style::SolidColor((0., 0., 0., 1.).into()),
      )));
    }
  }
}

impl LayoutAble for EditableText {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
    self.text.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: UIPosition) {
    self.text.set_position(position)
  }
}
