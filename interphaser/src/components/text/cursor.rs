use std::time::Instant;

use glyph_brush::ab_glyph::Font;

use crate::*;

pub struct Cursor {
  position: Option<CursorPositionInfo>,
  index: usize,
  update_timestamp: Instant,
}

struct CursorPositionInfo {
  // origin at top start
  position: UIPosition,
  height: f32,
}

pub(crate) enum CursorMove {
  Left,
  Right,
  Up,
  Down,
}

impl Cursor {
  pub fn new(index: usize) -> Self {
    Self {
      position: None,
      index,
      update_timestamp: Instant::now(),
    }
  }

  pub fn get_index(&self) -> usize {
    self.index
  }

  pub fn move_right(&mut self) {
    self.index += 1;
    self.position = None;
    self.update_timestamp = Instant::now();
  }

  pub fn move_left(&mut self) {
    self.index -= 1;
    self.position = None;
    self.update_timestamp = Instant::now();
  }

  pub fn set_index(&mut self, index: usize) {
    if index != self.index {
      self.position = None;
      self.update_timestamp = Instant::now();
    }
    self.index = index;
  }

  pub fn get_last_update_timestamp(&self) -> Instant {
    self.update_timestamp
  }

  pub fn notify_text_layout_changed(&mut self) {
    self.position = None;
  }

  fn get_position(&mut self, layout: &TextLayout, fonts: &FontManager) -> &CursorPositionInfo {
    self.position.get_or_insert_with(|| {
      let index = if self.index == 0 { 0 } else { self.index - 1 };
      if layout.is_empty() {
        // in this case, no content in editor,
        // we should place cursor at appropriate place
        // todo
        return CursorPositionInfo {
          position: (0., 0.).into(),
          height: 1.,
        };
      }

      let sg = &layout[index];
      let rect = fonts.get_font(sg.font_id).glyph_bounds(&sg.glyph);

      let height = rect.max.y - rect.min.y;
      let position = if self.index == 0 {
        (rect.min.x, rect.min.y)
      } else {
        (rect.max.x, rect.min.y)
      };
      CursorPositionInfo {
        position: position.into(),
        height,
      }
    })
  }

  pub fn create_quad(&mut self, layout: &TextLayout, fonts: &FontManager) -> Quad {
    let position = self.get_position(layout, fonts);
    Quad {
      x: position.position.x,
      y: position.position.y,
      width: 1.,
      height: position.height,
    }
  }
}
