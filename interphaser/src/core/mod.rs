mod layout;
pub use layout::*;

mod graphics;
pub use graphics::*;

mod event;
pub use event::*;

use crate::*;

pub trait Component: Eventable + Presentable + LayoutAble + Stream<Item = ()> + Unpin {}
impl<T> Component for T where T: Eventable + Presentable + LayoutAble + Stream<Item = ()> + Unpin {}

pub type BoxedUnpinStream<T> = Box<dyn Stream<Item = T> + Unpin>;
pub type BoxedUnpinFusedStream<T> = Box<dyn FusedStream<Item = T> + Unpin>;
