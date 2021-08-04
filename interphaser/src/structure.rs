use crate::UpdateCtx;

use super::Component;

pub struct If<T, C> {
  should_render: Box<dyn Fn(&T) -> bool>,
  func: Box<dyn Fn(&T) -> C>,
  inner: Option<C>,
}

impl<T, C> If<T, C>
where
  C: Component<T>,
{
  pub fn condition<F, SF>(should_render: SF, func: F) -> Self
  where
    SF: Fn(&T) -> bool + 'static,
    F: Fn(&T) -> C + 'static,
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
        self.inner = Some((self.func)(model));
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
  children: Vec<(T, C)>, // todo, should we optimize T to a simple key?
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
  T: 'static + PartialEq + Clone,
  C: Component<T>,
{
  fn update(&mut self, model: &Vec<T>, ctx: &mut UpdateCtx) {
    self.children = model
      .iter()
      .enumerate()
      .map(|(index, item)| {
        if let Some(previous) = self.children.iter().position(|cached| &cached.0 == item) {
          // move
          self.children.swap_remove(previous)
        } else {
          // create
          (item.clone(), (self.mapper)(item, index))
        }
      })
      .collect();
    // and not exist will be drop
  }

  fn event(&mut self, model: &mut Vec<T>, event: &mut crate::EventCtx<'_>) {
    self
      .children
      .iter_mut()
      .zip(model)
      .for_each(|((_, item), model)| item.event(model, event))
  }
}

type IterType<'a, C: 'static, T: 'static> = impl Iterator<Item = &'a mut C> + 'a;
impl<'a, T: 'static, C: 'static> IntoIterator for &'a mut For<T, C> {
  type Item = &'a mut C;
  type IntoIter = IterType<'a, C, T>;

  fn into_iter(self) -> IterType<'a, C, T> {
    self.children.iter_mut().map(|(_, c)| c)
  }
}
