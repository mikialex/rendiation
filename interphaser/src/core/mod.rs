mod layout;
use std::time::Duration;

pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

use crate::*;

pub trait Component<T, S: System = DefaultSystem> {
  fn event(&mut self, _model: &mut T, _event: &mut S::EventCtx<'_>) {}

  fn update(&mut self, _model: &T, _ctx: &mut S::UpdateCtx<'_>) {}
}

pub trait UIComponent<T>: Component<T> + Presentable + LayoutAble + 'static {}
impl<X, T> UIComponent<T> for X where X: Component<T> + Presentable + LayoutAble + 'static {}

pub trait BoxUIComponent<T>: UIComponent<T> + Sized {
  fn boxed(self) -> Box<dyn UIComponent<T>> {
    Box::new(self)
  }
}
impl<X, T> BoxUIComponent<T> for X where X: UIComponent<T> {}
impl<T> Component<T> for Box<dyn UIComponent<T>> {
  fn event(&mut self, model: &mut T, event: &mut <DefaultSystem as System>::EventCtx<'_>) {
    self.as_mut().event(model, event)
  }

  fn update(&mut self, model: &T, ctx: &mut <DefaultSystem as System>::UpdateCtx<'_>) {
    self.as_mut().update(model, ctx)
  }
}

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
