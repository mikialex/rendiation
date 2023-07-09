use crate::*;

pub struct Cursor {
  index: usize,
  position: Option<CursorPositionInfo>,
  update_timestamp: Instant,
  change: ViewUpdateNotifier,
}

impl Stream for Cursor {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.change.poll_next_unpin(cx)
  }
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
      change: Default::default(),
    }
  }

  pub fn get_index(&self) -> usize {
    self.index
  }

  pub fn move_right(&mut self) {
    self.index += 1;
    self.position = None;
    self.update_timestamp = Instant::now();
    self.change.notify();
  }

  pub fn move_left(&mut self) {
    self.index -= 1;
    self.position = None;
    self.update_timestamp = Instant::now();
    self.change.notify();
  }

  pub fn set_index(&mut self, index: usize) {
    if index != self.index {
      self.position = None;
      self.update_timestamp = Instant::now();
      self.change.notify();
    }
    self.index = index;
  }

  pub fn get_last_update_timestamp(&self) -> Instant {
    self.update_timestamp
  }

  pub fn notify_text_layout_changed(&mut self) {
    self.position = None;
    self.change.notify();
  }

  // todo fix!
  fn get_position(&mut self, layout: &TextLayoutRef) -> &CursorPositionInfo {
    let layout = layout.layout();
    let glyphs = &layout.glyphs;

    self.position.get_or_insert_with(|| {
      let index = if self.index == 0 { 0 } else { self.index - 1 };
      if glyphs.is_empty() {
        // in this case, no glyph in editor,
        // we should place cursor at appropriate place
        // todo
        return CursorPositionInfo {
          position: (0., 0.).into(),
          height: 1.,
        };
      }

      let glyph_index = layout
        .source
        .chars()
        .take(index)
        .filter(|c| !c.is_control())
        .count();

      let rect = &glyphs[glyph_index.min(glyphs.len() - 1)].2;

      let height = rect.right_bottom[1] - rect.left_top[1];
      let position = if self.index == 0 {
        (rect.left_top[0], rect.left_top[1])
      } else {
        (rect.right_bottom[0], rect.left_top[1])
      };
      CursorPositionInfo {
        position: position.into(),
        height,
      }
    })
  }

  pub fn create_quad(&mut self, layout: &TextLayoutRef) -> RectangleShape {
    let position = self.get_position(layout);
    RectangleShape {
      x: position.position.x,
      y: position.position.y,
      width: 1.,
      height: position.height,
    }
  }
}
