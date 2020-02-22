pub mod element;
pub mod event;
// pub mod lens;
pub mod renderer;
// pub use lens::*;
pub use renderer::*;
// pub mod t;
// use event::Event;

pub use element::*;
use rendiation::WGPURenderer;

pub struct GUI {
  fragment: ElementFragment,
  renderer: GUIRenderer,
}

impl GUI {
  pub fn new(renderer: & WGPURenderer) -> Self {
    GUI {
      fragment: ElementFragment::new(),
      renderer: GUIRenderer::new(renderer, (500., 500.))
    }
  }

  pub fn event() {

  }

  pub fn render(&self, renderer: &mut WGPURenderer) {

  }
}