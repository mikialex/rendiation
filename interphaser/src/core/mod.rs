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

pub trait ViewHelperExt: View {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
    let mut output = Default::default();
    self.request(&mut ViewRequest::Layout(LayoutProtocol::DoLayout {
      constraint,
      ctx,
      output: &mut output,
    }));
    output
  }

  fn set_position(&mut self, position: UIPosition) {
    self.request(&mut ViewRequest::Layout(LayoutProtocol::PositionAt(
      position,
    )));
  }

  fn event(&mut self, ctx: &mut EventCtx) {
    self.request(&mut ViewRequest::Event(ctx));
  }

  fn draw(&mut self, ctx: &mut PresentationBuilder) {
    self.request(&mut ViewRequest::Encode(ctx));
  }
}
impl<T: View + ?Sized> ViewHelperExt for T {}
