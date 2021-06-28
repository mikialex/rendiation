use std::{any::Any, marker::PhantomData};

use rendiation_algebra::Vec2;

pub trait Component: Clone + PartialEq + 'static {
  type State: PartialEq + Default;
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>);
}

#[derive(PartialEq, Clone)]
pub struct Button {
  label: String,
}

#[derive(Default, PartialEq)]
pub struct ButtonState {
  is_hovered: bool,
}

impl Component for Button {
  type State = ButtonState;
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    composer
      .push_primitive(Primitive::Quad)
      .push_primitive(Primitive::Text);
  }
}

#[derive(Default, PartialEq, Clone)]
pub struct FlexLayout {
  direction: bool,
}

impl Component for FlexLayout {
  type State = ();
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    // do nothing
  }
}

#[derive(Default, PartialEq, Clone)]
pub struct Counter;

#[derive(Default, PartialEq, Clone)]
pub struct CounterState {
  count: usize,
}

impl Component for Counter {
  type State = CounterState;
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    composer.children(FlexLayout { direction: false }.init(), |c| {
      c.child(
        Button {
          label: format!("add count{}", state.count),
        }
        .init::<Self>()
        .on(|s| s.count += 1),
      )
      .child(
        Button {
          label: format!("de count {}", state.count),
        }
        .init(),
      );
    });
  }
}

pub struct ComponentInit<T, P: Component> {
  init: T,
  events: Vec<Box<dyn Fn(&mut P::State)>>,
}

impl<T, P: Component> ComponentInit<T, P> {
  pub fn on(mut self, f: impl Fn(&mut P::State) + 'static) -> Self {
    self.events.push(Box::new(f));
    self
  }
}

pub trait ComponentInitAble: Sized {
  fn init<P: Component>(self) -> ComponentInit<Self, P> {
    ComponentInit {
      init: self,
      events: Vec::new(),
    }
  }
}
impl<T> ComponentInitAble for T {}

pub struct Composer<'a, P> {
  phantom: PhantomData<P>,
  primitives: &'a mut Vec<Primitive>,
  self_primitives: &'a mut Vec<Primitive>,
  new_props: Vec<Box<dyn Any>>,
  components: &'a mut Vec<Box<dyn ComponentInstance>>,
}

impl<'a, P: Component> Composer<'a, P> {
  pub fn children<T: Component, F: Fn(&mut Composer<P>)>(
    &mut self,
    props: ComponentInit<T, P>,
    children: F,
  ) -> &mut Self {
    let index = self.new_props.len();
    let component = if let Some(old_component) = self.components.get_mut(index) {
      if !old_component.patch(&props.init, self.primitives) {
        *old_component = Box::new(ComponentCell::<T, P>::new());
        old_component.patch(&props.init, self.primitives);
      };
      old_component
    } else {
      self.components.push(Box::new(ComponentCell::<T, P>::new()));
      self.components.last_mut().unwrap()
    };

    let (components, self_primitives) = component.compose_source();

    let mut composer = Composer {
      phantom: PhantomData,
      new_props: Vec::new(),
      primitives: self.primitives,
      components,
      self_primitives,
    };

    children(&mut composer);
    self
  }

  pub fn child<T: Component>(&mut self, props: ComponentInit<T, P>) -> &mut Self {
    let index = self.new_props.len();
    if let Some(old_component) = self.components.get_mut(index) {
      if !old_component.patch(&props.init, self.primitives) {
        *old_component = Box::new(ComponentCell::<T, P>::new());
        old_component.patch(&props.init, self.primitives);
      };
    } else {
      self.components.push(Box::new(ComponentCell::<T, P>::new()));
    };

    self
  }

  pub fn push_primitive(&mut self, p: Primitive) -> &mut Self {
    self.primitives.push(p);
    self
  }
}

struct ComponentCell<T: Component, P: Component> {
  state: T::State,
  state_changed: bool,
  last_props: Option<T>,
  event_handlers: Vec<Box<dyn Fn(&mut P::State)>>,
  children: Vec<Box<dyn ComponentInstance>>,
  self_primitives: Vec<Primitive>,
  layout_box: usize,
  is_active: bool,
}

impl<T: Component, P: Component> ComponentCell<T, P> {
  pub fn new() -> Self {
    Self {
      state: Default::default(),
      state_changed: true,
      last_props: None,
      event_handlers: Vec::new(),
      self_primitives: Vec::new(),
      children: Vec::new(),
      layout_box: 0,
      is_active: false,
    }
  }
}

trait ComponentInstance {
  fn patch(&mut self, props: &dyn Any, primitive_builder: &mut Vec<Primitive>) -> bool;
  fn compose_source(&mut self) -> (&mut Vec<Box<dyn ComponentInstance>>, &mut Vec<Primitive>);
  fn event(&mut self, event: &winit::event::Event<()>, parent: &mut dyn Any);
}

impl<T: Component, P: Component> ComponentInstance for ComponentCell<T, P> {
  fn patch(&mut self, props: &dyn Any, primitive_builder: &mut Vec<Primitive>) -> bool {
    if let Some(props) = props.downcast_ref::<T>() {
      if let Some(last_props) = &self.last_props {
        let props_changed = last_props != props;
        if props_changed || self.state_changed {
          // re render

          let mut composer = Composer {
            phantom: PhantomData,
            new_props: Vec::new(),
            primitives: primitive_builder,
            components: &mut self.children,
            self_primitives: &mut self.self_primitives,
          };

          props.render(&self.state, &mut composer);

          if props_changed {
            self.last_props = Some(props.clone())
          }
          self.state_changed = false;
        }
      }
      return true;
    } else {
      return false;
    }
  }

  fn compose_source(&mut self) -> (&mut Vec<Box<dyn ComponentInstance>>, &mut Vec<Primitive>) {
    (&mut self.children, &mut self.self_primitives)
  }

  fn event(&mut self, event: &winit::event::Event<()>, parent: &mut dyn Any) {
    // match event
    self.self_primitives.iter().for_each(|p| {
      if true {
        self
          .event_handlers
          .iter()
          .for_each(|f| f(parent.downcast_mut().unwrap()))
      }
    })
  }
}

#[derive(Debug)]
pub enum Primitive {
  Quad,
  Text,
}

impl Primitive {
  pub fn test_pointer_in(pointer: Vec2<f32>) -> bool {
    false
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UIRoot;

impl Component for UIRoot {
  type State = ();

  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    todo!()
  }
}

struct UI<T: Component> {
  component: ComponentCell<T, UIRoot>,
  primitive_cache: Vec<Primitive>,
}

impl<T: Component> UI<T> {
  pub fn new() -> Self {
    let component = ComponentCell::new();
    Self {
      component,
      primitive_cache: Vec::new(),
    }
  }

  pub fn render(&mut self) -> &Vec<Primitive> {
    self.primitive_cache.clear();
    self.component.patch(&(), &mut self.primitive_cache);
    &self.primitive_cache
  }

  pub fn event(&mut self, event: &winit::event::Event<()>) {
    self.component.event(event, &mut ())
  }
}

pub trait UIRenderer {
  fn render(&mut self, primitives: &Vec<Primitive>);
}

pub struct WebGPUxUIRenderer {
  //
}

#[test]
fn ui() {
  let mut ui = UI::<Counter>::new();
  ui.render();
}
