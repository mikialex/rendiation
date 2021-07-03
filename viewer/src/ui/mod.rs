use std::{
  any::Any,
  collections::HashMap,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

pub mod components;
pub mod examples;
pub mod renderer;

pub mod layout;
pub use layout::*;
pub mod rendering;
pub use rendering::*;

pub trait Component: Clone + PartialEq + Default + 'static {
  type State: PartialEq + Default;
  fn build(model: &mut Model<Self>, c: &mut Composer<Self>) {}

  // https://flutter.dev/docs/development/ui/layout/constraints
  fn request_layout_size(&self, state: &Self::State, constraint: &LayoutConstraint) -> LayoutSize {
    constraint.max()
  }
  fn layout_children(&self, state: &Self::State, ctx: &mut LayoutCtx) {
    let mut current_y = ctx.self_layout.y;
    ctx.children.iter_mut().for_each(|c| {
      let size = c.request_size(&LayoutConstraint::unlimited());
      c.set_layout(Layout {
        x: ctx.self_layout.x,
        y: current_y,
        size,
      });
      current_y += size.height;
    })
  }

  fn update(&self, state: &Self::State) {}

  fn render(&self, state: &Self::State) {}
}

pub struct Model<'a, C: Component> {
  state_and_props: &'a StateAndProps<C>,
  view_model: &'a mut HashMap<*const (), Box<dyn MemorizedViewModel<C>>>,
}

pub trait MemorizedViewModel<C: Component> {
  fn get_value(&mut self, data: &StateAndProps<C>) -> &dyn Any;
  fn has_changed(&self, data: &StateAndProps<C>) -> bool;
}

pub struct Memo<C: Component, T> {
  f: fn(&StateAndProps<C>) -> T,
  cache: Option<T>,
}

impl<C: Component, T: 'static + PartialEq> MemorizedViewModel<C> for Memo<C, T> {
  fn get_value(&mut self, data: &StateAndProps<C>) -> &dyn Any {
    self.cache.get_or_insert_with(|| (self.f)(data))
  }
  fn has_changed(&self, data: &StateAndProps<C>) -> bool {
    self.cache.as_ref().map_or(false, |c| c == &(self.f)(data))
  }
}

impl<C: Component, T> Memo<C, T> {
  pub fn new(f: fn(&StateAndProps<C>) -> T) -> Self {
    Self { f, cache: None }
  }
}

impl<'a, C: Component> Model<'a, C> {
  pub fn view<T: 'static + PartialEq>(&mut self, f: fn(&StateAndProps<C>) -> T) -> &T {
    let f_p = f as *const ();
    self
      .view_model
      .entry(f_p)
      .or_insert_with(|| Box::new(Memo::new(f)))
      .get_value(&self.state_and_props)
      .downcast_ref::<T>()
      .unwrap()
  }
  pub fn view_ref<F: Fn(&StateAndProps<C>) -> &T, T>(&self, f: F) -> &T {
    f(&self.state_and_props)
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
  new_props: Vec<Box<dyn Any>>,
  primitives: &'a mut Vec<Primitive>,
  target_children: &'a mut Vec<Box<dyn ComponentInstance>>,
}

impl<'a, P: Component> Composer<'a, P> {
  pub fn children<T, F>(&mut self, props: ComponentInit<T, P>, mut children: F) -> &mut Self
  where
    T: Component,
    F: FnMut(&mut Composer<P>),
  {
    let index = self.new_props.len();
    let component = if let Some(old_component) = self.target_children.get_mut(index) {
      if !old_component.patch(props.init) {
        *old_component = Box::new(ComponentCell::<T, P>::new(props.init.clone()));
        old_component.patch(props.init);
      };
      old_component
    } else {
      self
        .target_children
        .push(Box::new(ComponentCell::<T, P>::new(props.init.clone())));
      self.target_children.last_mut().unwrap()
    };

    let meta = component.meta_mut();

    let mut composer: Composer<P> = Composer {
      phantom: PhantomData,
      new_props: Vec::new(),
      target_children: &mut meta.out_children,
      primitives: &mut meta.primitives,
    };

    children(&mut composer);
    self
  }

  pub fn child<T: Component>(&mut self, props: ComponentInit<T, P>) -> &mut Self {
    self.children(props, |_| {})
  }

  pub fn draw_primitive(&mut self, p: Primitive) -> &mut Self {
    self.primitives.push(p);
    self
  }
}

pub struct StateAndProps<C: Component> {
  props: C,
  state: StateCell<C::State>,
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

pub struct ComponentCell<T: Component, P: Component> {
  data: StateAndProps<T>,
  event_handlers: Vec<Box<dyn Fn(&mut StateAndProps<P>)>>,
  view_model_cache: HashMap<*const (), Box<dyn MemorizedViewModel<T>>>,
  meta: ComponentMetaData,
}

pub struct ComponentMetaData {
  /// The direct components that belong to component internal.
  /// The real child component
  children: Vec<Box<dyn ComponentInstance>>,
  /// The direct components that append as the child of this component
  /// in outer view of component tree
  ///
  /// Maybe in future we can add multi "slot" support
  /// which need multi out_children groups
  out_children: Vec<Box<dyn ComponentInstance>>,
  /// The rendering primitive cache of this component
  ///
  /// Notice: this only contains the component it self's primitive,
  /// Neither the children nor the outer children
  ///
  /// component could not provide any primitive but still has layout
  primitives: Vec<Primitive>,
  layout: Layout,
  is_active: bool,
}

impl<T: Component, P: Component> ComponentCell<T, P> {
  pub fn new(props: T) -> Self {
    Self {
      data: StateAndProps {
        state: Default::default(),
        props,
      },
      event_handlers: Vec::new(),
      view_model_cache: HashMap::new(),
      meta: ComponentMetaData {
        primitives: Vec::new(),
        children: Vec::new(),
        out_children: Vec::new(),
        layout: Default::default(),
        is_active: false,
      },
    }
  }

  fn check_model_view_changed(&mut self) -> bool {
    for (_, mv) in &mut self.view_model_cache {
      if mv.has_changed(&self.data) {
        return true;
      }
    }
    false
  }

  fn traverse_owned_child(
    &mut self,
    f: &mut impl FnMut(&mut dyn ComponentInstance, &mut StateAndProps<T>),
  ) {
    fn traverse_outer<T: Component>(
      com: &mut dyn ComponentInstance,
      root: &mut StateAndProps<T>,
      f: &mut impl FnMut(&mut dyn ComponentInstance, &mut StateAndProps<T>),
    ) {
      f(com, root);
      com
        .meta_mut()
        .out_children
        .iter_mut()
        .for_each(|c| traverse_outer(c.as_mut(), root, f))
    }

    self
      .meta
      .children
      .iter_mut()
      .for_each(|c| traverse_outer(c.as_mut(), &mut self.data, f))
  }
}

trait ComponentInstance {
  fn patch(&mut self, props: &dyn Any) -> bool;
  fn meta_mut(&mut self) -> &mut ComponentMetaData;
  fn meta(&self) -> &ComponentMetaData;
  fn event(&mut self, event: &winit::event::Event<()>, parent_data: &mut dyn Any);
  fn render(&mut self, result: &mut Vec<Primitive>);
}

impl<T: Component, P: Component> ComponentInstance for ComponentCell<T, P> {
  fn patch(&mut self, props: &dyn Any) -> bool {
    // check this component should rebuild caused by component type changed
    if let Some(props) = props.downcast_ref::<T>() {
      // if component type not changed, we diff the cached props and see if it state
      // changed. if any of it changed, we should rebuild it, diff it new component tree
      let props_changed = &self.data.props != props;
      if props_changed || self.data.state.changed || self.check_model_view_changed() {
        let mut composer: Composer<T> = Composer {
          phantom: PhantomData,
          new_props: Vec::new(),
          // we're rebuild the current component instance's own children
          target_children: &mut self.meta.children,
          primitives: &mut self.meta.primitives,
        };

        let mut model = Model {
          state_and_props: &self.data,
          view_model: &mut self.view_model_cache,
        };

        T::build(&mut model, &mut composer);

        if props_changed {
          self.data.props = props.clone()
        }
        self.data.state.changed = false;
      }
      return true;
    } else {
      return false;
    }
  }

  fn meta_mut(&mut self) -> &mut ComponentMetaData {
    &mut self.meta
  }

  fn meta(&self) -> &ComponentMetaData {
    &self.meta
  }

  fn event(&mut self, event: &winit::event::Event<()>, parent_data: &mut dyn Any) {
    let mut parent_data = parent_data.downcast_mut::<StateAndProps<P>>().unwrap();

    // todo match event
    self.meta.primitives.iter().for_each(|p| {
      if true {
        self.event_handlers.iter().for_each(|f| f(&mut parent_data))
      }
    });

    self.traverse_owned_child(&mut |c, p| c.event(event, p))
  }

  fn render(&mut self, result: &mut Vec<Primitive>) {
    result.extend(self.meta.primitives.clone().into_iter());
    self.traverse_owned_child(&mut |c, _| c.render(result))
  }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct UIRoot;

impl Component for UIRoot {
  type State = UIRootState;
}

#[derive(PartialEq, Clone)]
pub struct UIRootState {
  size: LayoutSize,
}

impl Default for UIRootState {
  fn default() -> Self {
    Self {
      size: LayoutSize {
        width: 500.,
        height: 300.,
      },
    }
  }
}

struct UI<T: Component> {
  root: StateAndProps<UIRoot>,
  component: ComponentCell<T, UIRoot>,
  primitive_cache: Vec<Primitive>,
}

impl<T: Component> UI<T> {
  pub fn new() -> Self {
    let component = ComponentCell::new(T::default());
    let root = StateAndProps {
      props: UIRoot,
      state: Default::default(),
    };
    Self {
      root,
      component,
      primitive_cache: Vec::new(),
    }
  }

  pub fn update(&mut self) {
    self.component.patch(&());
  }

  pub fn render(&mut self) -> &Vec<Primitive> {
    self.primitive_cache.clear();
    self.component.render(&mut self.primitive_cache);
    &self.primitive_cache
  }

  fn size(&self) -> LayoutSize {
    self.root.state.size
  }

  fn set_size(&mut self, size: LayoutSize) -> &mut Self {
    self.root.state.size = size;
    self
  }

  pub fn event(&mut self, event: &winit::event::Event<()>) {
    self.component.event(event, &mut self.root)
  }
}
