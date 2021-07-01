use std::{
  any::Any,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

pub mod components;
pub mod examples;
pub mod renderer;
pub use renderer::*;

pub trait Component: Clone + PartialEq + Default + 'static {
  type State: PartialEq + Default;
  fn build(&self, state: &Self::State, composer: &mut Composer<Self>) {}

  // https://flutter.dev/docs/development/ui/layout/constraints
  fn request_layout_size(&self, state: &Self::State, constraint: &LayoutConstraint) -> LayoutSize {
    constraint.max()
  }
  fn layout_children(&self, state: &Self::State, self_layout: &Layout, children: &mut LayoutCtx) {
    todo!()
  }

  fn update(&self, state: &Self::State) {}

  fn render(&self, state: &Self::State) {}
}

pub trait LayoutAble {
  fn request_size(&self, constraint: &LayoutConstraint) -> LayoutSize;
  fn update(&mut self) -> &mut Layout;
}

pub struct LayoutCtx<'a> {
  children: [&'a mut dyn LayoutAble],
}

pub struct LayoutConstraint {
  pub width_min: f32,
  pub width_max: f32,
  pub height_min: f32,
  pub height_max: f32,
}

impl LayoutConstraint {
  pub fn max(&self) -> LayoutSize {
    LayoutSize {
      width: self.width_max,
      height: self.height_max,
    }
  }
}

pub struct LayoutSize {
  pub width: f32,
  pub height: f32,
}

pub struct Layout {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl Default for Layout {
  fn default() -> Self {
    Self {
      x: 0.,
      y: 0.,
      width: 0.,
      height: 0.,
    }
  }
}

pub struct ComponentInit<'a, T, P: Component> {
  init: &'a T,
  events: Vec<Box<dyn Fn(&mut StateAndProps<P>)>>,
}

impl<'a, T, P: Component> ComponentInit<'a, T, P> {
  pub fn on(mut self, f: impl Fn(&mut StateAndProps<P>) + 'static) -> Self {
    self.events.push(Box::new(f));
    self
  }
}

pub trait ComponentInitAble: Sized {
  fn init<P: Component>(&self) -> ComponentInit<Self, P> {
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
  pub fn children<T, F>(&mut self, props: ComponentInit<T, P>, children: F) -> &mut Self
  where
    T: Component,
    F: Fn(&mut Composer<P>),
  {
    let index = self.new_props.len();
    let component = if let Some(old_component) = self.components.get_mut(index) {
      if !old_component.patch(props.init, self.primitives) {
        *old_component = Box::new(ComponentCell::<T, P>::new(props.init.clone()));
        old_component.patch(props.init, self.primitives);
      };
      old_component
    } else {
      self
        .components
        .push(Box::new(ComponentCell::<T, P>::new(props.init.clone())));
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
      if !old_component.patch(props.init, self.primitives) {
        *old_component = Box::new(ComponentCell::<T, P>::new(props.init.clone()));
        old_component.patch(props.init, self.primitives);
      };
    } else {
      self
        .components
        .push(Box::new(ComponentCell::<T, P>::new(props.init.clone())));
    };

    self
  }

  pub fn draw_primitive(&mut self, p: Primitive) -> &mut Self {
    self.primitives.push(p);
    self
  }
}

pub struct StateAndProps<'a, C: Component> {
  props: &'a C,
  state: &'a mut StateCell<C::State>,
}

pub struct StateCell<T> {
  state: T,
  changed: bool,
}

impl<T: Default> Default for StateCell<T> {
  fn default() -> Self {
    Self {
      state: Default::default(),
      changed: true,
    }
  }
}

impl<T> Deref for StateCell<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.state
  }
}

impl<T> DerefMut for StateCell<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.state
  }
}

struct ComponentCell<T: Component, P: Component> {
  state: StateCell<T::State>,
  props: T,
  event_handlers: Vec<Box<dyn Fn(&mut StateAndProps<P>)>>,
  children: Vec<Box<dyn ComponentInstance>>,
  self_primitives: Vec<Primitive>,
  layout: Layout,
  is_active: bool,
}

struct ComponentData {}

impl<T: Component, P: Component> ComponentCell<T, P> {
  pub fn new(props: T) -> Self {
    Self {
      state: Default::default(),
      props,
      event_handlers: Vec::new(),
      self_primitives: Vec::new(),
      children: Vec::new(),
      layout: Default::default(),
      is_active: false,
    }
  }
}

trait ComponentInstance {
  fn patch(&mut self, props: &dyn Any, primitive_builder: &mut Vec<Primitive>) -> bool;
  fn compose_source(&mut self) -> (&mut Vec<Box<dyn ComponentInstance>>, &mut Vec<Primitive>);
  fn event(&mut self, event: &winit::event::Event<()>, parent: &dyn Any);
}

impl<T: Component, P: Component> ComponentInstance for ComponentCell<T, P> {
  fn patch(&mut self, props: &dyn Any, primitive_builder: &mut Vec<Primitive>) -> bool {
    if let Some(props) = props.downcast_ref::<T>() {
      let props_changed = &self.props != props;
      if props_changed || self.state.changed {
        // re render

        let mut composer = Composer {
          phantom: PhantomData,
          new_props: Vec::new(),
          primitives: primitive_builder,
          components: &mut self.children,
          self_primitives: &mut self.self_primitives,
        };

        props.build(&self.state, &mut composer);

        if props_changed {
          self.props = props.clone()
        }
        self.state.changed = false;
      }
      return true;
    } else {
      return false;
    }
  }

  fn compose_source(&mut self) -> (&mut Vec<Box<dyn ComponentInstance>>, &mut Vec<Primitive>) {
    (&mut self.children, &mut self.self_primitives)
  }

  fn event(&mut self, event: &winit::event::Event<()>, parent: &dyn Any) {
    let state_and_props = StateAndProps {
      props: parent.downcast_ref::<P>().unwrap(),
      state: &mut self.state,
    };
    // match event
    self.self_primitives.iter().for_each(|p| {
      if true {
        self
          .event_handlers
          .iter()
          .for_each(|f| f(&mut state_and_props))
      }
    })
  }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct UIRoot;

impl Component for UIRoot {
  type State = ();
}

struct UI<T: Component> {
  component: ComponentCell<T, UIRoot>,
  primitive_cache: Vec<Primitive>,
}

impl<T: Component> UI<T> {
  pub fn new() -> Self {
    let component = ComponentCell::new(T::default());
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
