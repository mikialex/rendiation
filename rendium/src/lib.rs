pub mod component;
pub mod element;
pub mod renderer;
use crate::element::Event;
use crate::renderer::GUIRenderer;
use component::*;

pub struct GUI<T: Component<T>> {
  state: T,
  root: ComponentInstance<T>,
  renderer: GUIRenderer
}

impl<T: Component<T>> GUI<T> {
  pub fn event(&mut self, event: &Event) {
    self.root.event(event, &mut self.state);
  }

  pub fn update(&mut self){

  }

  pub fn render(&mut self) {
    // do render
  }
}
