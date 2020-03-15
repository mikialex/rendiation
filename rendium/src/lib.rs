pub mod application;
pub mod component;
pub mod element;
pub mod event;
pub mod window;
// pub mod lens;
pub mod renderer;

pub use application::*;
pub use window::*;
pub use event::*;
// pub use lens::*;
pub use renderer::*;


pub use winit;
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

  pub fn event(event: Event) {
    
  }

  pub fn render(&mut self, renderer: &mut WGPURenderer) {
    self.fragment.render(renderer, &mut self.renderer);
  }
}