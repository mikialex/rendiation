use crate::{FontManager, UpdateCtx};

mod unit;
pub use unit::*;
mod types;
pub use types::*;

pub struct LayoutCtx<'a> {
  pub fonts: &'a FontManager,
}

#[derive(Default)]
pub struct LayoutResult {
  pub size: LayoutSize,
  pub baseline_offset: f32,
}

pub trait LayoutAble {
  fn layout(&mut self, constraint: LayoutConstraint, _ctx: &mut LayoutCtx) -> LayoutResult {
    LayoutResult {
      size: constraint.min(),
      baseline_offset: 0.,
    }
  }
  fn set_position(&mut self, _position: UIPosition) {}
}

#[derive(Default)]
pub struct LayoutSource<T> {
  value: T,
  changed: bool,
}

impl<T> LayoutSource<T> {
  pub fn new(value: T) -> Self {
    Self {
      value,
      changed: true,
    }
  }
  pub fn set(&mut self, value: impl Into<T>) {
    self.value = value.into();
    self.changed = true;
  }

  pub fn mutate(&mut self) -> &mut T {
    self.changed = true;
    &mut self.value
  }

  pub fn get(&mut self) -> &T {
    &self.value
  }

  pub fn refresh(&mut self, layout: &mut LayoutUnit, ctx: &mut UpdateCtx) {
    if self.changed {
      layout.request_layout(ctx)
    }
    self.changed = false;
  }
}
