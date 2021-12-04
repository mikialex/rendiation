use crate::*;

pub struct LayoutUnit {
  previous_constrains: LayoutConstraint,
  /// relative to parent top left
  pub relative_position: UIPosition,
  /// relative to screen top left
  pub absolute_position: UIPosition,
  pub size: LayoutSize,
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
  pub fn check_attach(&mut self, ctx: &mut UpdateCtx) {
    if !self.attached {
      ctx.request_layout();
      self.attached = true;
    }
  }

  pub fn or_layout_change(&mut self, ctx: &mut UpdateCtx) {
    self.need_update |= ctx.layout_changed;
  }

  pub fn request_layout(&mut self, ctx: &mut UpdateCtx) {
    self.need_update = true;
    ctx.request_layout();
  }

  pub fn skipable(&mut self, new_constraint: LayoutConstraint) -> bool {
    let constraint_changed = new_constraint != self.previous_constrains;
    if constraint_changed {
      self.previous_constrains = new_constraint;
    }
    self.need_update |= constraint_changed;
    let result = !self.need_update;
    self.need_update = false;
    result
  }

  pub fn set_relative_position(&mut self, position: UIPosition) {
    self.relative_position = position;
  }

  pub fn update_world(&mut self, world_offset: UIPosition) {
    self.absolute_position.x = self.relative_position.x + world_offset.x;
    self.absolute_position.y = self.relative_position.y + world_offset.y;
  }

  pub fn into_quad(&self) -> Quad {
    Quad {
      x: self.absolute_position.x,
      y: self.absolute_position.y,
      width: self.size.width,
      height: self.size.height,
    }
  }
}
