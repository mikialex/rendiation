use super::Element;
use crate::event::Event;

struct ElementsTree<T> {
  elements: Vec<Box<dyn Element<T>>>,
}

impl<T> ElementsTree<T> {
  pub fn event(&mut self, event: &Event) {}
}
