use std::{any::Any, marker::PhantomData};

pub trait Component: Default + PartialEq + 'static {
  type Props: PartialEq + Clone;
  fn render(&self, props: &Self::Props, composer: &mut Composer<Self>);
}

#[derive(PartialEq, Clone)]
pub struct ButtonProps {
  label: String,
}

#[derive(Default, PartialEq)]
pub struct Button {
  is_hovered: bool,
}

impl Component for Button {
  type Props = ButtonProps;
  fn render(&self, props: &Self::Props, composer: &mut Composer<Self>) {
    composer.push_primitive();
  }
}

#[derive(Default, PartialEq)]
pub struct FlexLayout;
#[derive(PartialEq, Clone)]
pub struct FlexLayoutProps {
  direction: bool,
}

impl Component for FlexLayout {
  type Props = FlexLayoutProps;
  fn render(&self, props: &Self::Props, composer: &mut Composer<Self>) {
    //
  }
}

#[derive(Default, PartialEq)]
pub struct Counter {
  count: usize,
}

impl Component for Counter {
  type Props = ();
  fn render(&self, props: &Self::Props, composer: &mut Composer<Self>) {
    composer
      .push::<Button>(ButtonProps {
        label: format!("add count{}", self.count),
      })
      .push::<Button>(ButtonProps {
        label: format!("de count {}", self.count),
      });
  }
}

pub struct Composer<'a, P> {
  phantom: PhantomData<P>,
  //   primitives: Vec<usize>,
  components: Vec<Box<dyn Any>>,
  old_components: &'a mut Vec<Box<dyn ComponentInstance>>,
}

impl<'a, P: Component> Composer<'a, P> {
  pub fn push<T: Component>(&mut self, props: T::Props) -> &mut Self {
    let index = self.components.len();
    if let Some(old_component) = self.old_components.get_mut(index) {
      old_component.patch(&props);
    } else {
      self
        .old_components
        .push(Box::new(ComponentCell::<T, P>::new()))
    }
    self
  }

  pub fn push_primitive(&mut self) -> &mut Self {
    self
  }
}

struct ComponentCell<T: Component, P> {
  state: T,
  state_changed: bool,
  last_props: Option<T::Props>,
  event_handlers: Vec<Box<dyn Fn(P)>>,
  children: Vec<Box<dyn ComponentInstance>>,
  layout_box: usize,
}

impl<T: Component, P> ComponentCell<T, P> {
  pub fn new() -> Self {
    Self {
      state: Default::default(),
      state_changed: true,
      last_props: None,
      event_handlers: Vec::new(),
      children: Vec::new(),
      layout_box: 0,
    }
  }
}

trait ComponentInstance {
  fn patch(&mut self, props: &dyn Any);
}

impl<T: Component, P> ComponentInstance for ComponentCell<T, P> {
  fn patch(&mut self, props: &dyn Any) {
    if let Some(props) = props.downcast_ref::<T::Props>() {
      if let Some(last_props) = &self.last_props {
        let props_changed = last_props != props;
        if props_changed || self.state_changed {
          // re render

          let mut composer = Composer {
            phantom: PhantomData,
            components: Vec::new(),
            old_components: &mut self.children,
          };

          self.state.render(props, &mut composer);

          if props_changed {
            self.last_props = Some(props.clone())
          }
          self.state_changed = false;
        }
      }
    }
  }
}

struct UI<T: Component> {
  component: ComponentCell<T, ()>,
  primitive_cache: Vec<usize>,
}

impl<T: Component> UI<T> {
  pub fn new() -> Self {
    let component = ComponentCell::new();
    Self {
      component,
      primitive_cache: Vec::new(),
    }
  }

  pub fn update(&mut self) -> &Vec<usize> {
    todo!()
  }
}

#[test]
fn ui() {
  let mut ui = UI::<Counter>::new();
  ui.update();
}
