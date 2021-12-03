use std::marker::PhantomData;

use crate::*;

pub trait ComponentExt<T>: Component<T> + Sized {
  fn extend<A: ComponentAbility<T, Self>>(self, ability: A) -> Ability<T, Self, A> {
    Ability::new(self, ability)
  }
  fn lens<S, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
    LensWrap::new(self, lens)
  }
}

impl<X, T> ComponentExt<T> for X where X: Component<T> + Sized {}

pub trait ComponentAbilityExt<T, C>: ComponentAbility<T, C> + Sized {
  fn wrap(self, inner: C) -> Ability<T, C, Self> {
    Ability::new(inner, self)
  }
}

impl<C, X, T> ComponentAbilityExt<T, C> for X where X: ComponentAbility<T, C> + Sized {}

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

pub trait ComponentAbility<T, C> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx);
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C);
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
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C);
}

impl<T, C, A: PresentableAbility<C>> Presentable for Ability<T, C, A> {
  fn render(&mut self, builder: &mut crate::PresentationBuilder) {
    self.ability.render(builder, &mut self.inner)
  }
}

pub trait LayoutAbility<C> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    _ctx: &mut LayoutCtx,
    _inner: &mut C,
  ) -> LayoutResult {
    LayoutResult {
      size: constraint.min(),
      baseline_offset: 0.,
    }
  }
  fn set_position(&mut self, _position: UIPosition, _inner: &mut C) {}
}

impl<T, C, A: LayoutAbility<C>> LayoutAble for Ability<T, C, A> {
  fn layout(
    &mut self,
    constraint: crate::LayoutConstraint,
    ctx: &mut LayoutCtx,
  ) -> crate::LayoutResult {
    self.ability.layout(constraint, ctx, &mut self.inner)
  }

  fn set_position(&mut self, position: crate::UIPosition) {
    self.ability.set_position(position, &mut self.inner)
  }
}

pub trait HotAreaPassBehavior<C> {
  fn is_point_in(&self, _point: crate::UIPosition, _inner: &C) -> bool {
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
