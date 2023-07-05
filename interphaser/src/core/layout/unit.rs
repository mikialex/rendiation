use crate::*;

/// Each layout is hold by the components in component tree.
///
/// The component tree actually not a layout tree. The real
/// layout tree is composed by the LayoutUnit.
pub struct LayoutUnit {
  previous_constrains: LayoutConstraint,
  /// relative to parent top left
  pub relative_position: UIPosition,
  /// relative to screen top left
  pub absolute_position: UIPosition,
  pub size: UISize,
  pub baseline_offset: f32,
  pub attached: bool,
  pub need_update: bool,
}

impl Default for LayoutUnit {
  fn default() -> Self {
    Self {
      previous_constrains: Default::default(),
      relative_position: Default::default(),
      size: Default::default(),
      absolute_position: Default::default(),
      baseline_offset: 0.,
      attached: false,
      need_update: true,
    }
  }
}

impl LayoutUnit {
  pub fn set_relative_position(&mut self, position: UIPosition) {
    self.relative_position = position;
  }

  pub fn update_world(&mut self, world_offset: UIPosition) {
    self.absolute_position.x = self.relative_position.x + world_offset.x;
    self.absolute_position.y = self.relative_position.y + world_offset.y;
  }

  pub fn into_quad(&self) -> RectangleShape {
    RectangleShape {
      x: self.absolute_position.x,
      y: self.absolute_position.y,
      width: self.size.width,
      height: self.size.height,
    }
  }
}
