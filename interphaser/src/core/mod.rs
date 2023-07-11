mod layout;
pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

use crate::*;

pub type BoxedUnpinStream<T> = Box<dyn Stream<Item = T> + Unpin>;
pub type BoxedUnpinFusedStream<T> = Box<dyn FusedStream<Item = T> + Unpin>;

pub enum ViewRequest<'a, 'b, 'c> {
  Event(&'a mut EventCtx<'b>),
  Layout(LayoutProtocol<'b, 'c>),
  Encode(&'a mut PresentationBuilder<'b>),
}

pub trait View: Stream<Item = ()> + Unpin {
  fn request(&mut self, detail: &mut ViewRequest);
}
