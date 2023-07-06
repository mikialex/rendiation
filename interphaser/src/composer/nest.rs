use crate::*;

pub trait ComponentNestExt: Component + Sized {
  fn nest_in<A: EventableNested<Self>>(self, ability: A) -> NestedComponent<Self, A> {
    NestedComponent::new(self, ability)
  }
}

impl<X> ComponentNestExt for X where X: Component + Sized {}

pub trait ComponentAbilityExt<C>: Sized {
  fn nest_over(self, inner: C) -> NestedComponent<C, Self> {
    NestedComponent::new(inner, self)
  }
}

impl<C, X> ComponentAbilityExt<C> for X where X: Sized {}

pub struct NestedComponent<C, A> {
  inner: C,
  ability: A,
}

impl<C, A> NestedComponent<C, A> {
  pub fn new(inner: C, ability: A) -> Self {
    Self { inner, ability }
  }
}

pub trait EventableNested<C> {
  fn event(&mut self, event: &mut EventCtx, inner: &mut C);
}

impl<C, A> Eventable for NestedComponent<C, A>
where
  C: Eventable,
  A: EventableNested<C>,
{
  fn event(&mut self, event: &mut EventCtx) {
    self.ability.event(event, &mut self.inner);
  }
}

pub trait PresentableNested<C> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C);
}

impl<C, A: PresentableNested<C>> Presentable for NestedComponent<C, A> {
  fn render(&mut self, builder: &mut crate::PresentationBuilder) {
    self.ability.render(builder, &mut self.inner)
  }
}

pub trait LayoutAbleNested<C> {
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

impl<C, A: LayoutAbleNested<C>> LayoutAble for NestedComponent<C, A> {
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

pub trait HotAreaNested<C> {
  fn is_point_in(&self, _point: crate::UIPosition, _inner: &C) -> bool {
    false
  }
}

impl<C, A> HotAreaProvider for NestedComponent<C, A>
where
  A: HotAreaNested<C>,
{
  fn is_point_in(&self, point: crate::UIPosition) -> bool {
    self.ability.is_point_in(point, &self.inner)
  }
}
