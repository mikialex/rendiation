use std::marker::PhantomData;

use super::Component;

pub struct Ability<T, C, A> {
  inner: C,
  ability: A,
  phantom: PhantomData<T>,
}

impl<T, C, A> Ability<T, C, A> {
  pub fn new(inner: C, ability: A) -> Self {
    Self {
      inner,
      ability,
      phantom: PhantomData,
    }
  }
}

pub trait ComponentAbility<T, C: Component<T>> {
  fn update(&mut self, model: &T, inner: &mut C) {
    inner.update(model);
  }
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T, C, A> Component<T> for Ability<T, C, A>
where
  C: Component<T>,
  A: ComponentAbility<T, C>,
{
  fn update(&mut self, model: &T) {
    self.ability.update(model, &mut self.inner);
  }
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>) {
    self.ability.event(model, event, &mut self.inner);
  }
}
