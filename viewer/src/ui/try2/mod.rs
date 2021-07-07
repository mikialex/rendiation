use std::marker::PhantomData;

pub trait Component<T> {
  fn event(&mut self, state: &mut T) {}

  fn update(&mut self, model: &T) {}
}

struct ComponentCell<C> {
  com: C,
}

pub enum ValueCell<T, U> {
  Static(T),
  Dynamic(DynamicValue<T, U>),
}
impl<T, U> ValueCell<T, U> {
  pub fn update(&mut self, ctx: &U) {
    todo!()
  }
}

pub struct DynamicValue<T, U> {
  fun: Box<dyn Fn(&U) -> T>,
  value: T,
}

pub struct Text<T> {
  content: ValueCell<String, T>,
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

pub struct Container<T, C> {
  width: f32,
  height: f32,
  inner: C,
  phantom: PhantomData<T>,
}

struct Todo {
  items: TodoItems,
}

struct TodoItems {
  name: String,
}

fn build_todo() -> impl Component<Todo> {
  Flex::<Todo> {
    children: Vec::new(),
  }
}

struct Flex<T> {
  children: Vec<Box<dyn Component<T>>>,
}

impl<T> Component<T> for Flex<T> {}

struct Button;

impl<T> Component<T> for Button {}
