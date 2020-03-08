pub mod element;
pub mod event;
// pub mod lens;
pub mod renderer;
// pub use lens::*;
pub use renderer::*;
// pub mod t;
// use event::Event;

pub mod component;

pub use element::*;
use rendiation::WGPURenderer;

pub struct GUI {
  fragment: ElementFragment,
  pub renderer: GUIRenderer,
}

impl GUI {
  pub fn new(renderer: &mut WGPURenderer, size: (f32, f32)) -> Self {
    GUI {
      fragment: ElementFragment::new(),
      renderer: GUIRenderer::new(renderer, size)
    }
  }

  pub fn event() {
    
  }

  pub fn render(&mut self, renderer: &mut WGPURenderer) {
    self.fragment.render(renderer, &mut self.renderer);
  }
}