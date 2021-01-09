use winit::event::Event;

mod component;
mod document;
mod element;
mod window_state;
pub use component::*;
pub use document::*;
pub use element::*;
pub use window_state::*;

pub struct GUI<App> {
  window_states: WindowState,
  document: Document,
  app: App,
}

impl<App> GUI<App> {
  pub fn new(app: App) -> Self {
    GUI {
      window_states: WindowState::new(),
      document: Document::new(),
      app,
    }
  }

  pub fn event(event: Event<()>) {}

  pub fn render<Backend>(&mut self, renderer: &mut Backend) {
    // self.fragment.render(renderer, &mut self.renderer);

    // self.renderer.update_to_screen(renderer, target);
    // renderer.submit();
  }
}
