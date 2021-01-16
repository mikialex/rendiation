use std::{any::Any, cell::RefCell, marker::PhantomData};

use super::{Element, ElementHandle};
use arena::{Arena, Handle};
use arena_tree::ArenaTree;

trait Component: Sized {
  type State: Default;
  type Props;
  type Event;
  fn build(state: &Self::State, props: &Self::Props) -> ComponentContent<Self>;
}

pub struct ViewBuilder<T> {
  phantom: PhantomData<T>,
  // children: Vec<DocumentElement<T>>
}
pub fn h<T>() -> ViewBuilder<T> {
  todo!()
}
impl<T> ViewBuilder<T> {
  fn on(&mut self) {
    //
  }

  fn child<VT>(&mut self, v: ViewBuilder<VT>) {
    //
  }
}

struct ComponentContent<T: Component> {
  root_element: ElementHandle,
  tree: ArenaTree<DocumentElement<T>>,
  events: EventDispatcher<T::Event>,
}

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

enum ComponentElementCell<T: Component> {
  HadBuild(Box<dyn ComponentInstance<Props = T::Props>>),
  // NotBuild(Box<dyn Fn(T) -> >)
}

type DisplayList = Vec<usize>;

pub trait ComponentInstance {
  type Props;
  /// receive event from outside, emit listener and modify self state
  fn event(&self, props: &Self::Props);
  fn update(&self, props: &Self::Props);
  fn render(&self, list: &mut DisplayList);
}

struct ComponentInstanceContainer<T: Component> {
  current_states: T::State,
  last_states: Option<T::State>,
  cached_props: Option<T::Props>,
  content: ComponentContent<T>,
}

impl<T> ComponentInstance for ComponentInstanceContainer<T>
where
  T: Component,
  T::Props: PartialEq,
  T::State: PartialEq,
{
  type Props = T::Props;
  fn event(&self, props: &T::Props) {
    todo!()
  }
  fn update(&self, props: &T::Props) {
    // if props not changed, we don't update
    // if self.cached_props.eq(props) {
    //   return;
    // }
    let new_view = T::build(&self.current_states, props);
    // diff and patch
  }
  fn render(&self, list: &mut DisplayList) {
    todo!()
  }
}
