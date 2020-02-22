pub mod element;
pub mod event;
// pub mod lens;
pub mod renderer;
// pub use lens::*;
pub use renderer::*;
// pub mod t;
// use event::Event;

pub use element::*;

struct GUI {
  fragment: ElementFragment,
}