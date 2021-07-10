use super::{Component, Passthrough};

pub struct If<T, C> {
  should_render: Box<dyn Fn(&T) -> bool>,
  func: Box<dyn Fn() -> C>,
  inner: Option<C>,
}

impl<T, C> If<T, C>
where
  C: Component<T>,
{
  pub fn condition<F, SF>(should_render: SF, func: F) -> Self
  where
    SF: Fn(&T) -> bool + 'static,
    F: Fn() -> C + 'static,
  {
    Self {
      should_render: Box::new(should_render),
      func: Box::new(func),
      inner: None,
    }
  }
}

impl<T, C> Component<T> for If<T, C>
where
  C: Component<T>,
{
  fn update(&mut self, model: &T) {
    if (self.should_render)(model) {
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

pub struct For<T, C> {
  children: Vec<C>,
  mapper: Box<dyn Fn(&T, usize) -> C>,
}

impl<T, C> For<T, C>
where
  C: Component<T>,
{
  pub fn by<F>(mapper: F) -> Self
  where
    F: Fn(&T, usize) -> C + 'static,
  {
    Self {
      children: Vec::new(),
      mapper: Box::new(mapper),
    }
  }
}

impl<'a, IT, T, C> Component<IT> for For<T, C>
where
  T: 'static,
  IT: Iterator<Item = &'a T>,
  C: Component<T>,
{
  fn update(&mut self, model: &IT) {
    todo!()
  }
}

impl<T, C> Passthrough<T> for For<T, C>
where
  C: Component<T>,
{
  fn visit(&self, mut f: impl FnMut(&dyn Component<T>)) {
    self.children.iter().for_each(|c| f(c as &dyn Component<T>))
  }

  fn mutate(&mut self, mut f: impl FnMut(&mut dyn Component<T>)) {
    self
      .children
      .iter_mut()
      .for_each(|c| f(c as &mut dyn Component<T>))
  }
}
