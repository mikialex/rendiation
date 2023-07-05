use crate::*;

pub trait ComponentExt: Component + Sized {
  fn extend<A: ComponentAbility<Self>>(self, ability: A) -> Ability<Self, A> {
    Ability::new(self, ability)
  }
}

impl<X> ComponentExt for X where X: Component + Sized {}

pub trait ComponentAbilityExt<C>: ComponentAbility<C> + Sized {
  fn wrap(self, inner: C) -> Ability<C, Self> {
    Ability::new(inner, self)
  }
}

impl<C, X> ComponentAbilityExt<C> for X where X: ComponentAbility<C> + Sized {}

pub struct Ability<C, A> {
  inner: C,
  ability: A,
}

impl<C, A> Ability<C, A> {
  pub fn new(inner: C, ability: A) -> Self {
    Self { inner, ability }
  }
}

pub trait ComponentAbility<C> {
  fn event(&mut self, event: &mut EventCtx, inner: &mut C);
}

impl<C, A> Component for Ability<C, A>
where
  C: Component,
  A: ComponentAbility<C>,
{
  fn event(&mut self, event: &mut EventCtx) {
    self.ability.event(event, &mut self.inner);
  }
}

pub trait PresentableAbility<C> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C);
}

impl<C, A: PresentableAbility<C>> Presentable for Ability<C, A> {
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

impl<C, A: LayoutAbility<C>> LayoutAble for Ability<C, A> {
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

impl<C, A> HotAreaProvider for Ability<C, A>
where
  A: HotAreaPassBehavior<C>,
{
  fn is_point_in(&self, point: crate::UIPosition) -> bool {
    self.ability.is_point_in(point, &self.inner)
  }
}
