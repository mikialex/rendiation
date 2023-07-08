use std::time::Duration;

use futures::Stream;
use winit::event::VirtualKeyCode;

use crate::*;

#[derive(Clone)]
pub enum TextEditMessage {
  ContentChange(String),
  KeyboardInput(VirtualKeyCode),
}

pub struct EditableText {
  text: Text,
  editing: String,
  cursor: Option<Cursor>,
  pub events: EventSource<TextEditMessage>,
  pub update_source: Option<BoxedUnpinStream<String>>,
  pub focus_input: Option<BoxedUnpinStream<()>>,
}

impl EditableText {
  pub fn set_focus(&mut self, focus: impl Stream<Item = ()> + Unpin + 'static) {
    self.focus_input = Some(Box::new(focus))
  }
  pub fn set_update_source(&mut self, source: impl Stream<Item = String> + Unpin + 'static) {
    self.update_source = Some(Box::new(source))
  }

  fn focus(&mut self) {
    if self.cursor.is_none() {
      self.cursor = Cursor::new(self.editing.len()).into();
    }
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

  fn handle_input(&mut self, key: winit::event::VirtualKeyCode) {
    use winit::event::VirtualKeyCode::*;
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
    let mut r = EditableText {
      text: self,
      editing,
      cursor: None,
      events: Default::default(),
      focus_input: Default::default(),
      update_source: Default::default(),
    };

    let updater = r.events.unbound_listen().filter_map_sync(|v| match v {
      TextEditMessage::ContentChange(v) => Some(v),
      _ => None,
    });

    // note, this will override the previous user set updater
    r.text.set_updater(updater);
    r
  }
}

impl Stream for EditableText {
  type Item = ();
  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    if let Some(update_source) = &mut self.update_source {
      if let Poll::Ready(Some(content)) = update_source.poll_next_unpin(cx) {
        self.editing = content;
        self
          .events
          .emit(&TextEditMessage::ContentChange(self.editing.clone()));
      }
    }

    if let Some(inputs) = &mut self.focus_input {
      if let Poll::Ready(Some(())) = inputs.poll_next_unpin(cx) {
        self.focus();
      }
    }
    let mut view_changed = false;
    // if cursor created(when focused), we will not miss the init poll(here)
    if let Some(cursor) = &mut self.cursor {
      view_changed |= cursor.poll_next_unpin(cx).is_ready();
    }

    view_changed |= self.text.poll_next_unpin(cx).is_ready();
    if view_changed {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

impl Eventable for EditableText {
  fn event(&mut self, ctx: &mut EventCtx) {
    self.text.event(ctx);

    use winit::event::*;

    match ctx.event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::KeyboardInput { input, .. } => {
          if let Some(virtual_keycode) = input.virtual_keycode {
            if input.state == ElementState::Pressed {
              self.handle_input(virtual_keycode);
              self
                .events
                .emit(&TextEditMessage::KeyboardInput(virtual_keycode));
            }
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if let (MouseButton::Left, ElementState::Pressed) = (button, state) {
            self.update_cursor_by_click(ctx.states.mouse_position, ctx.fonts, ctx.texts);
          }
        }
        WindowEvent::ReceivedCharacter(char) => {
          self.insert_at_cursor(*char);
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

impl Presentable for EditableText {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.clamp_cursor_position();
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
