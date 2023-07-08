use crate::{FontManager, TextCache};

mod unit;
pub use unit::*;
mod types;
pub use types::*;
mod alignment;
pub use alignment::*;

pub struct LayoutCtx<'a> {
  pub fonts: &'a FontManager,
  pub text: &'a TextCache,
}

#[derive(Default)]
pub struct LayoutResult {
  pub size: UISize,
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
