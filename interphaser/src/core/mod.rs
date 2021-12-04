mod layout;
use std::time::Duration;

pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

mod text;
pub use text::*;

use crate::*;

pub trait Component<T, S: System = DefaultSystem> {
  fn event(&mut self, _model: &mut T, _event: &mut S::EventCtx<'_>) {}

  fn update(&mut self, _model: &T, _ctx: &mut S::UpdateCtx<'_>) {}
}

pub trait UIComponent<T>: Component<T> + Presentable + LayoutAble + 'static {}
impl<X, T> UIComponent<T> for X where X: Component<T> + Presentable + LayoutAble + 'static {}

pub trait System {
  type EventCtx<'a>;
  type UpdateCtx<'a>;
}

pub struct DefaultSystem {}

impl System for DefaultSystem {
  type EventCtx<'a> = EventCtx<'a>;
  type UpdateCtx<'a> = UpdateCtx<'a>;
}

pub struct UpdateCtx<'a> {
  /// the incremental time stamp since the application started
  pub time_stamp: Duration,
  pub layout_changed: bool,
  pub fonts: &'a FontManager,
  pub last_frame_perf_info: &'a PerformanceInfo,
}

impl<'a> UpdateCtx<'a> {
  pub fn request_layout(&mut self) {
    self.layout_changed = true;
  }
}
