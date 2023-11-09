use interphaser::*;

mod widget;
pub use widget::*;

mod terminal;
pub use terminal::*;

mod viewer_view;
pub use viewer_view::*;

pub fn create_app() -> impl View {
  Flex::column().wrap(flex_group().child(Child::flex(viewer(), 1.)))
}
