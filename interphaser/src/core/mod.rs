mod layout;
pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

mod inc;
pub use inc::*;

pub trait Eventable {
  fn event(&mut self, event: &mut EventCtx);
}

pub trait Component: Eventable + Presentable + LayoutAble {}
impl<T> Component for T where T: Component + Presentable + LayoutAble {}
