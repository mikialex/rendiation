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

pub enum LayoutProtocol<'a, 'b> {
  DoLayout {
    constraint: LayoutConstraint,
    ctx: &'a mut LayoutCtx<'b>,
    output: &'a mut LayoutResult,
  },
  PositionAt(UIPosition),
}
