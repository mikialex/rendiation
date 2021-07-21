use std::marker::PhantomData;

use crate::{EventCtx, HotAreaProvider, Presentable, PresentationBuilder, UpdateCtx};

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
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    inner.update(model, ctx);
  }
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T, C, A> Component<T> for Ability<T, C, A>
where
  C: Component<T>,
  A: ComponentAbility<T, C>,
{
  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.ability.update(model, &mut self.inner, ctx);
  }
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    self.ability.event(model, event, &mut self.inner);
  }
}

pub trait PresentableAbility<C> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C);
}

impl<T, C, A: PresentableAbility<C>> Presentable for Ability<T, C, A> {
  fn render(&self, builder: &mut crate::PresentationBuilder) {
    self.ability.render(builder, &self.inner)
  }
}

pub trait HotAreaPassBehavior<C> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    false
  }
}

impl<T, C, A> HotAreaProvider for Ability<T, C, A>
where
  A: HotAreaPassBehavior<C>,
{
  fn is_point_in(&self, point: crate::UIPosition) -> bool {
    self.ability.is_point_in(point, &self.inner)
  }
}
