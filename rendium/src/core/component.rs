use std::{any::Any, cell::RefCell};

use super::{Element, ElementHandle};
use arena::{Arena, Handle};
use arena_tree::ArenaTree;

trait Component: Sized {
  type State: Default;
  type Props;
  type Event;
  fn build(state: &Self::State, props: &Self::Props) -> ComponentContent<Self>;
}

pub struct ComponentBuilder {}

struct ComponentContent<T: Component> {
  root_element: ElementHandle,
  tree: ArenaTree<DocumentElement<T>>,
  events: EventDispatcher<T::Event>,
}

// impl<T: Component> ComponentContent<T> {
//   pub fn on(&mut self) {

//   }
// }

pub struct EventDispatcher<T> {
  listeners: Arena<Box<dyn FnMut(&mut T)>>,
}

pub type EventListenerHandle<T> = Handle<Box<dyn FnMut(&mut T)>>;

impl<T> EventDispatcher<T> {
  pub fn new() -> Self {
    Self {
      listeners: Arena::new(),
    }
  }

  pub fn add<L: FnMut(&mut T) + 'static>(&mut self, listener: L) -> EventListenerHandle<T> {
    self.listeners.insert(Box::new(listener))
  }
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
