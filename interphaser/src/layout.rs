use crate::{FontManager, Quad, UpdateCtx};

pub struct LayoutCtx<'a> {
  pub fonts: &'a FontManager,
}

pub trait LayoutAble {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutSize {
    constraint.min()
  }
  fn set_position(&mut self, _position: UIPosition) {}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraint {
  pub width_min: f32,
  pub width_max: f32,
  pub height_min: f32,
  pub height_max: f32,
}

impl Default for LayoutConstraint {
  fn default() -> Self {
    Self::unlimited()
  }
}

impl LayoutConstraint {
  pub fn unlimited() -> Self {
    Self {
      width_min: 0.,
      width_max: 0.,
      height_min: f32::INFINITY,
      height_max: f32::INFINITY,
    }
  }
  pub fn from_max(size: LayoutSize) -> Self {
    Self {
      width_min: 0.,
      width_max: size.width,
      height_min: 0.,
      height_max: size.height,
    }
  }
  pub fn max(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_max,
      height: self.height_max,
    }
  }
  pub fn min(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_min,
      height: self.height_min,
    }
  }
  pub fn clamp(&self, size: LayoutSize) -> LayoutSize {
    LayoutSize {
      width: size.width.clamp(self.width_min, self.width_max),
      height: size.height.clamp(self.height_min, self.height_max),
    }
  }

  pub fn set_max_width(&mut self, width: f32) {
    self.width_max = width;
    self.width_max = self.width_max.max(self.width_min);
  }

  pub fn set_max_height(&mut self, height: f32) {
    self.height_max = height;
    self.height_max = self.height_max.max(self.height_min);
  }

  pub fn consume_width(&self, width: f32) -> Self {
    Self {
      width_min: self.width_min - width,
      width_max: self.width_max - width,
      ..*self
    }
    .min_zero()
  }

  pub fn consume_height(&self, height: f32) -> Self {
    Self {
      height_min: self.height_min - height,
      height_max: self.height_max - height,
      ..*self
    }
    .min_zero()
  }

  pub fn min_zero(&self) -> Self {
    Self {
      width_min: self.width_min.min(0.),
      width_max: self.width_max.min(0.),
      height_min: self.height_min.min(0.),
      height_max: self.height_max.min(0.),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutSize {
  pub width: f32,
  pub height: f32,
}

impl LayoutSize {
  pub fn new(width: f32, height: f32) -> Self {
    Self { width, height }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UIPosition {
  pub x: f32,
  pub y: f32,
}

/// Layout coordinate use x => right. y => down (same as web API canvas2D);
pub struct Layout {
  pub position: UIPosition,
  pub size: LayoutSize,
}

impl Default for Layout {
  fn default() -> Self {
    Self {
      position: UIPosition { x: 0., y: 0. },
      size: LayoutSize {
        width: 0.,
        height: 0.,
      },
    }
  }
}

pub struct LayoutUnit {
  previous_constrains: LayoutConstraint,
  pub size: LayoutSize,
  pub position: UIPosition,
  pub attached: bool,
  pub need_update: bool,
}

impl Default for LayoutUnit {
  fn default() -> Self {
    Self {
      previous_constrains: Default::default(),
      size: Default::default(),
      position: Default::default(),
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

  pub fn skipable(&mut self, new_constraint: LayoutConstraint) -> bool {
    let constraint_changed = new_constraint != self.previous_constrains;
    if constraint_changed {
      self.previous_constrains = new_constraint;
    }
    self.need_update |= constraint_changed;
    !self.need_update
  }

  pub fn into_quad(&self) -> Quad {
    Quad {
      x: self.position.x,
      y: self.position.y,
      width: self.size.width,
      height: self.size.height,
    }
  }
}
