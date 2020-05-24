pub mod application;
pub mod component;
pub mod element;
pub mod event;
pub mod window;
// pub mod lens;
pub mod data;
pub mod renderer;

pub use application::*;
pub use event::*;
pub use window::*;
// pub use lens::*;
pub use renderer::*;

pub use element::*;
pub use winit;

use rendiation::{render_target::ScreenRenderTarget, WGPURenderer};
pub use generational_arena::*;

pub struct GUI {
  fragment: ElementFragment,
  pub renderer: GUIRenderer,
}

impl GUI {
  pub fn new(renderer: &mut WGPURenderer, size: (f32, f32), screen: &ScreenRenderTarget) -> Self {
    GUI {
      fragment: ElementFragment::new(),
      renderer: GUIRenderer::new(renderer, size, screen),
    }
  }

  pub fn event(event: Event) {}

  pub fn render(&mut self, renderer: &mut WGPURenderer) {
    self.fragment.render(renderer, &mut self.renderer);
  }
}
