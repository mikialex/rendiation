use std::ops::{Deref, DerefMut};

use glyph_brush::ab_glyph::Font;

use crate::*;

pub struct EditableText {
  text: Text,
  cursor: Option<Cursor>,
}

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
  fn update_cursor_by_click(&mut self, position: UIPosition, fonts: &FontManager) {
    let layout = self.text.get_text_layout(fonts);
    let rect = layout
      .iter()
      .map(|sg| fonts.get_font(sg.font_id).glyph_bounds(&sg.glyph))
      .enumerate()
      .find(|(_, rect)| {
        position.x >= rect.min.x
          && position.x <= rect.max.x
          && position.y >= rect.min.y
          && position.y <= rect.max.y
      });

    if let Some((index, rect)) = rect {
      let height = rect.max.y - rect.min.y;
      let (text_index, position) = if position.x >= (rect.max.x + rect.min.x) / 2. {
        (index + 1, (rect.max.x, rect.min.y))
      } else {
        (index, (rect.min.x, rect.min.y))
      };

      self.cursor = Cursor {
        position: position.into(),
        height,
        text_index,
      }
      .into()
    }
  }
}

impl Text {
  pub fn editable(self) -> EditableText {
    EditableText {
      text: self,
      cursor: None,
    }
  }
}

pub struct Cursor {
  // top_start
  position: UIPosition,
  height: f32,
  text_index: usize,
}

impl Cursor {
  pub fn create_quad(&self) -> Quad {
    Quad {
      x: self.position.x,
      y: self.position.y,
      width: 2.,
      height: self.height,
    }
  }
}

impl<T> Component<T> for EditableText {
  fn event(&mut self, model: &mut T, ctx: &mut EventCtx) {
    self.text.event(model, ctx);

    use winit::event::*;

    match ctx.event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::KeyboardInput { input, .. } => {
          // todo handle keyborad input
          // modify text, emit change
          ctx.custom_event.push_event(1);
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if let (MouseButton::Left, ElementState::Pressed) = (button, state) {
            self.update_cursor_by_click(ctx.states.mouse_position, &ctx.fonts)
          }
        }
        _ => {}
      },
      _ => {}
    }
  }

  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.text.update(model, ctx)
  }
}

impl Presentable for EditableText {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.text.render(builder);
    if let Some(cursor) = &self.cursor {
      builder.present.primitives.push(Primitive::Quad((
        cursor.create_quad(),
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
