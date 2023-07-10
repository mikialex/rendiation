use std::ops::DerefMut;

use crate::*;

/// The helper trait to link different component together
pub trait ComponentNestExt: Sized {
  fn nest_in<A>(self, outer: A) -> NestedComponent<Self, A>
  where
    A: ComponentNester<Self>,
  {
    NestedComponent::new(self, outer)
  }
  fn wrap<C>(self, inner: C) -> NestedComponent<C, Self>
where
    // Self: ComponentNester<C>, 
    // todo check if compiler bug?
  {
    NestedComponent::new(inner, self)
  }
}
impl<X> ComponentNestExt for X where X: Sized {}

/// Combinator structure
pub struct NestedComponent<C, A> {
  inner: C,
  outer: A,
}

impl<C, A> NestedComponent<C, A> {
  pub fn new(inner: C, outer: A) -> Self {
    Self { inner, outer }
  }
}

/// This is the helper trait to wire up bounds to provide easier compiler error message
pub trait ComponentNester<C>:
  EventableNester<C> + LayoutAbleNester<C> + PresentableNester<C> + ReactiveUpdateNester<C>
{
}
impl<C, T> ComponentNester<C> for T where
  T: EventableNester<C> + LayoutAbleNester<C> + PresentableNester<C> + ReactiveUpdateNester<C>
{
}

pub trait ReactiveUpdateNester<C> {
  fn poll_update_inner(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>>;
}

impl<C, A> Stream for NestedComponent<C, A>
where
  C: Stream<Item = ()> + Unpin,
  A: ReactiveUpdateNester<C> + Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    let mut view_changed = false;
    view_changed |= Pin::new(&mut this.outer)
      .poll_update_inner(cx, &mut this.inner)
      .is_ready();
    view_changed |= this.inner.poll_next_unpin(cx).is_ready();
    if view_changed {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

pub trait EventableNester<C> {
  fn event(&mut self, event: &mut EventCtx, inner: &mut C);
}

impl<C, A> Eventable for NestedComponent<C, A>
where
  C: Eventable,
  A: EventableNester<C>,
{
  fn event(&mut self, event: &mut EventCtx) {
    self.outer.event(event, &mut self.inner);
  }
}

pub trait PresentableNester<C> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C);
}

impl<C, A: PresentableNester<C>> Presentable for NestedComponent<C, A> {
  fn render(&mut self, builder: &mut crate::PresentationBuilder) {
    self.outer.render(builder, &mut self.inner)
  }
}

pub trait LayoutAbleNester<C> {
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

impl<C, A: LayoutAbleNester<C>> LayoutAble for NestedComponent<C, A> {
  fn layout(
    &mut self,
    constraint: crate::LayoutConstraint,
    ctx: &mut LayoutCtx,
  ) -> crate::LayoutResult {
    self.outer.layout(constraint, ctx, &mut self.inner)
  }

  fn set_position(&mut self, position: crate::UIPosition) {
    self.outer.set_position(position, &mut self.inner)
  }
}

pub trait HotAreaNester<C> {
  fn is_point_in(&self, _point: crate::UIPosition, _inner: &C) -> bool {
    false
  }
}

impl<C, A> HotAreaProvider for NestedComponent<C, A>
where
  A: HotAreaNester<C>,
{
  fn is_point_in(&self, point: crate::UIPosition) -> bool {
    self.outer.is_point_in(point, &self.inner)
  }
}
