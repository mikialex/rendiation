use interphaser::*;

pub mod widget;
pub use widget::*;

pub mod terminal;
pub use terminal::*;

pub mod viewer_view;
pub use viewer_view::*;

pub fn create_app() -> impl View {
  Flex::column().wrap(flex_group().child(Child::flex(viewer(), 1.)))
}
