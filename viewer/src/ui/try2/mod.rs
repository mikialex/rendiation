use std::marker::PhantomData;

mod example;

pub trait Component<T> {
  fn event(&mut self, state: &mut T, event: &winit::event::Event<()>) {}

  fn update(&mut self, model: &T) {}
}

pub enum ValueCell<T, U> {
  Static(T),
  Dynamic(DynamicValue<T, U>),
}
impl<T, U> ValueCell<T, U> {
  pub fn update(&mut self, ctx: &U) -> &T {
    todo!()
  }
}

pub struct DynamicValue<T, U> {
  fun: Box<dyn Fn(&U) -> T>,
  value: Option<T>,
}

pub struct Text<T> {
  content: ValueCell<String, T>,
}

impl<T> Into<ValueCell<String, T>> for &str {
  fn into(self) -> ValueCell<String, T> {
    todo!()
  }
}

impl<T> Text<T> {
  pub fn new(content: impl Into<ValueCell<String, T>>) -> Self {
    todo!()
  }
}

impl<T> Component<T> for Text<T> {
  fn update(&mut self, model: &T) {
    self.content.update(model);
  }
}

pub struct ClickArea<T, C> {
  inner: C,
  phantom: PhantomData<T>,
}

impl<T, C: Component<T>> ComponentExt<T> for C {}

trait ComponentExt<T>: Component<T> + Sized {
  fn sized(self, width: f32, height: f32) -> Container<T, Self> {
    Container {
      width,
      height,
      inner: self,
      phantom: PhantomData,
    }
  }
  fn on(self, func: impl Fn(&mut T) + 'static) -> EventHandler<T, Self> {
    EventHandler {
      handler: Box::new(func),
      inner: self,
    }
  }
}

pub struct Container<T, C> {
  width: f32,
  height: f32,
  inner: C,
  phantom: PhantomData<T>,
}

impl<T, C: Component<T>> Component<T> for Container<T, C> {}

struct EventHandler<T, C> {
  handler: Box<dyn Fn(&mut T)>,
  inner: C,
}

impl<T, C: Component<T>> Component<T> for EventHandler<T, C> {}

fn button<T>(label: &str) -> impl Component<T> {
  Text::new(label).sized(300., 100.).on(|c| {})
  // .border(1)
  //   .on_click(|e|)
  //   .lens()
}

struct If<T> {
  should_render: ValueCell<bool, T>,
  func: Box<dyn Fn() -> Box<dyn Component<T>>>,
  inner: Option<Box<dyn Component<T>>>,
}

impl<T> If<T> {
  pub fn condition<C, F>(should_render: impl Into<ValueCell<bool, T>>, func: F) -> If<T>
  where
    C: Component<T> + 'static,
    F: Fn() -> C + 'static,
  {
    Self {
      should_render: should_render.into(),
      func: Box::new(move || Box::new(func())),
      inner: None,
    }
  }
}

impl<T> Component<T> for If<T> {
  fn update(&mut self, model: &T) {
    if *self.should_render.update(model) {
      if let Some(inner) = &mut self.inner {
        inner.update(model);
      } else {
        self.inner = Some((self.func)());
      }
    } else {
      self.inner = None;
    }
  }
}
