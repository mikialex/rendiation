use crate::UpdateCtx;

use super::Component;

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
  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    if (self.should_render)(model) {
      if let Some(inner) = &mut self.inner {
        inner.update(model, ctx);
      } else {
        self.inner = Some((self.func)());
      }
    } else {
      self.inner = None;
    }
  }

  fn event(&mut self, model: &mut T, event: &mut crate::EventCtx) {
    if let Some(inner) = &mut self.inner {
      inner.event(model, event)
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

impl<'a, T, C> Component<Vec<T>> for For<T, C>
where
  T: 'static,
  C: Component<T>,
{
  fn update(&mut self, model: &Vec<T>, ctx: &mut UpdateCtx) {
    todo!()
  }
}
