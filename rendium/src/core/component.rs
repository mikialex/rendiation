use std::{any::Any, cell::RefCell, marker::PhantomData};

// use super::{Element, ElementHandle};
use arena::{Arena, Handle};
use arena_tree::ArenaTree;
use rendiation_algebra::Vec2;

type Event = winit::event::Event<'static, ()>;

trait Component: Sized + 'static {
  type State: Default;
  type Props;
  type Event;
  fn build(state: &Self::State, props: &Self::Props) -> DocumentTree<Self>;
}

trait Element: 'static {
  /// decide if itself respond to a mouse event by mouse point
  fn is_point_in(&self, point: Vec2<f32>) -> bool;
  fn render(&self, list: &mut DisplayList);
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
  ComponentElement(Box<dyn DocumentUnit<Props = T::Props>>),
}

struct ElementCell<T: Component, E: Element> {
  element: E,
  events: EventDispatcher<T::Event>,
}

impl<T: Component, E: Element> DocumentUnit for ElementCell<T, E> {
  type Props = T::Props;

  fn event(&self, props: &Self::Props, event: &Event) {
    // if self.is_point_in()
    todo!()
  }

  fn update(&mut self, props: &Self::Props) {}

  fn render(&self, list: &mut DisplayList) {
    self.element.render(list)
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

enum ComponentElementCell<T: Component, S: Component> {
  HasBuilt(ComponentInstanceContainer<S>),
  ToBuild(Box<dyn Fn(T) -> S>),
}

impl<T: Component, S: Component> DocumentUnit for ComponentElementCell<T, S> {
  type Props = T::Props;

  fn event(&self, props: &Self::Props, event: &Event) {
    todo!()
  }

  fn update(&mut self, props: &Self::Props) {
    todo!()
  }

  fn render(&self, list: &mut DisplayList) {
    todo!()
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

type DisplayList = Vec<usize>;

pub trait DocumentUnit: Any {
  type Props;
  /// receive event from outside, emit listener and modify self state
  fn event(&self, props: &Self::Props, event: &Event);
  fn update(&mut self, props: &Self::Props);
  fn render(&self, list: &mut DisplayList);
  // fn patch(&mut self, other: dyn DocumentUnit<Props = Self::Props>);
  fn as_any(&self) -> &dyn Any;
}

struct DocumentTree<T: Component> {
  root_element: Handle<Box<dyn DocumentUnit<Props = T::Props>>>,
  tree: ArenaTree<Box<dyn DocumentUnit<Props = T::Props>>>,
}

impl<T: Component> DocumentTree<T> {
  pub fn patch(&mut self, new: Self) {
    // diff and patch!
  }
}

struct ComponentInstanceContainer<T: Component> {
  events: EventDispatcher<T::Event>,
  current_states: T::State,
  last_states: Option<T::State>,
  cached_props: Option<T::Props>,
  content: DocumentTree<T>,
}

impl<T> DocumentUnit for ComponentInstanceContainer<T>
where
  T: Component,
  T::Props: PartialEq,
  T::State: PartialEq,
{
  type Props = T::Props;
  fn event(&self, props: &T::Props, event: &Event) {
    todo!()
  }
  fn update(&mut self, props: &T::Props) {
    // if props not changed, we don't update
    // if self.cached_props.eq(props) {
    //   return;
    // }
    let new_view = T::build(&self.current_states, props);
    self.content.patch(new_view);
  }
  fn render(&self, list: &mut DisplayList) {
    todo!()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
