mod layout;
pub use layout::*;

mod rendering;
pub use rendering::*;

mod event;
pub use event::*;

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
  pub time_stamp: u64,
  pub layout_changed: bool, // todo private
  pub fonts: &'a FontManager,
  pub last_frame_perf_info: &'a PerformanceInfo,
}

impl<'a> UpdateCtx<'a> {
  pub fn request_layout(&mut self) {
    self.layout_changed = true;
  }
}
