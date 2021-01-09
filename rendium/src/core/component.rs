use std::{any::Any, cell::RefCell};

use super::{Element, ElementHandle};
use arena_tree::ArenaTree;

trait Component: Sized {
  type State: Default;
  type Props;
  type Event;
  fn build(state: &Self::State, props: &Self::Props) -> ComponentContent<Self>;
}

struct ComponentContent<T: Component> {
  root_element: ElementHandle,
  tree: ArenaTree<DocumentElement<T>>,
}

enum DocumentElement<T: Component> {
  PrimitiveElement(Box<dyn Element>),
  ComponentElement(Box<dyn ComponentInstance<Props = T::Props>>),
}

type DisplayList = Vec<usize>;

pub trait ComponentInstance {
  type Props;
  fn event(&self, props: &Self::Props);
  fn update(&self, props: &Self::Props);
  fn render(&self, list: &mut DisplayList);
}

struct ComponentInstanceContainer<T: Component> {
  states: RefCell<T::State>,
  content: ComponentContent<T>,
}

impl<T: Component> ComponentInstance for ComponentInstanceContainer<T> {
  type Props = T::Props;
  fn event(&self, props: &T::Props) {
    todo!()
  }
  fn update(&self, props: &T::Props) {
    todo!()
  }
  fn render(&self, list: &mut DisplayList) {
    todo!()
  }
}
