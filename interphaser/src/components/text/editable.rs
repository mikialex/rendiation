use std::time::Duration;

use futures::Stream;
use winit::keyboard::KeyCode;

use crate::*;

#[derive(Clone)]
pub enum TextEditMessage {
  ContentChange(String),
  KeyboardInput(KeyCode),
}

pub type EditableText = NestedView<impl View + AsMut<Text>, TextEditing>;

pub struct TextEditing {
  editing: String,
  cursor: Option<Cursor>,
  pub events: EventSource<TextEditMessage>,
}

impl TextEditing {
  pub fn focus(&mut self) {
    if self.cursor.is_none() {
      self.cursor = Cursor::new(self.editing.len()).into();
    }
  }

  pub fn set_text(&mut self, new_content: String) {
    self.editing = new_content;
    self
      .events
      .emit(&TextEditMessage::ContentChange(self.editing.clone()));
  }

  // when model updated by user side
  // cursor position maybe overflow the text length
  // so we simply clamp it
  fn clamp_cursor_position(&mut self) {
    if let Some(cursor) = &mut self.cursor {
      cursor.set_index(cursor.get_index().clamp(0, self.editing.len()));
    }
  }

  fn update_cursor_by_click(
    &mut self,
    position: UIPosition,
    fonts: &FontManager,
    texts: &mut TextCache,
    text: &mut Text,
  ) {
    let layout = text.get_text_layout(fonts, texts);
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

  fn insert_at_cursor(&mut self, c: char) {
    if c.is_control() {
      return;
    }
    if let Some(cursor) = &mut self.cursor {
      let index = cursor.get_index();
      self.editing.insert(index, c);
      self
        .events
        .emit(&TextEditMessage::ContentChange(self.editing.clone()));
      cursor.notify_text_layout_changed();
      cursor.move_right();
    }
  }

  fn delete_at_cursor(&mut self) {
    let content = &mut self.editing;
    if let Some(cursor) = &mut self.cursor {
      if cursor.get_index() == 0 {
        // if cursor at first, cant delete
        return;
      }
      content.remove(cursor.get_index() - 1);
      self
        .events
        .emit(&TextEditMessage::ContentChange(self.editing.clone()));
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
          if cursor.get_index() != self.editing.len() {
            cursor.move_right();
          }
        }
        CursorMove::Up => {} // todo
        CursorMove::Down => {}
      }
    }
  }

  fn handle_input(&mut self, key: KeyCode) {
    match key {
      Left => self.move_cursor(CursorMove::Left),
      Up => self.move_cursor(CursorMove::Up),
      Right => self.move_cursor(CursorMove::Right),
      Down => self.move_cursor(CursorMove::Down),
      Back => self.delete_at_cursor(),
      _ => {}
    }
  }
}

impl Text {
  pub fn editable(self) -> EditableText {
    let editing = self.get_content().into();
    let edit = TextEditing {
      editing,
      cursor: None,
      events: Default::default(),
    };

    let text_change = edit.events.unbound_listen().filter_map_sync(|v| match v {
      TextEditMessage::ContentChange(v) => Some(v),
      _ => None,
    });

    self.react(text_change.bind(Text::set_text)).nest_in(edit)
  }
}

impl Stream for TextEditing {
  type Item = ();
  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut view_changed = false;
    // todo fix if cursor created(when focused), we will not miss the init poll(here)
    if let Some(cursor) = &mut self.cursor {
      view_changed |= cursor.poll_next_unpin(cx).is_ready();
    }

    if view_changed {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

impl TextEditing {
  fn event(&mut self, ctx: &mut EventCtx, text: &mut Text) {
    use winit::event::*;

    match ctx.event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::KeyboardInput { event, .. } => {
          if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
            if event.state == ElementState::Pressed {
              self.handle_input(code);
              self.events.emit(&TextEditMessage::KeyboardInput(code));
            }
          }
          if let Some(text) = event.text {
            self.insert_at_cursor(text.chars().next().unwrap());
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if let (MouseButton::Left, ElementState::Pressed) = (button, state) {
            self.update_cursor_by_click(ctx.states.mouse_position, ctx.fonts, ctx.texts, text);
          }
        }
        _ => {}
      },
      _ => {}
    }
  }
}

fn blink_show(dur: Duration) -> bool {
  let time = dur.as_millis();
  time % 1000 > 500
}

impl<C: AsMut<Text>> ViewNester<C> for TextEditing {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    let text = inner.as_mut();
    match detail {
      ViewRequest::Event(event) => self.event(event, text),
      ViewRequest::Encode(builder) => {
        self.clamp_cursor_position();
        text.draw(builder);
        if let Some(cursor) = &mut self.cursor {
          if blink_show(cursor.get_last_update_timestamp().elapsed()) {
            return;
          }

          let layout = text.get_text_layout(builder.fonts, builder.texts);
          builder.present.primitives.push(Primitive::Quad((
            cursor.create_quad(layout),
            Style::SolidColor((0., 0., 0., 1.).into()),
          )));
        }
      }
      _ => text.request(detail),
    }
  }
}
