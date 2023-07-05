mod layout;
pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

mod inc;
pub use inc::*;

pub trait Component {
  fn event(&mut self, event: &mut EventCtx);
}

// pub trait Component: Component + Presentable + LayoutAble + 'static {}
// impl<X, T> Component for X where X: Component + Presentable + LayoutAble + 'static {}

// pub trait System {
//   type EventCtx<'a>;
//   type UpdateCtx<'a>;
// }

// pub struct DefaultSystem {}

// impl System for DefaultSystem {
//   type EventCtx<'a> = EventCtx<'a>;
//   type UpdateCtx<'a> = UpdateCtx<'a>;
// }

// pub struct UpdateCtx<'a> {
//   /// the incremental time stamp since the application started
//   pub time_stamp: Duration,
//   pub layout_changed: bool,
//   pub fonts: &'a FontManager,
//   pub last_frame_perf_info: &'a PerformanceInfo,
// }

// impl<'a> UpdateCtx<'a> {
//   pub fn request_layout(&mut self) {
//     self.layout_changed = true;
//   }
// }
