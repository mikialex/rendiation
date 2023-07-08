use crate::*;

pub trait ComponentNestExt: Sized {
  fn nest_in<A>(self, outer: A) -> NestedComponent<Self, A> {
    NestedComponent::new(self, outer)
  }
  fn nest_over<C>(self, inner: C) -> NestedComponent<C, Self> {
    NestedComponent::new(inner, self)
  }
}

impl<X> ComponentNestExt for X where X: Sized {}

pub struct NestedComponent<C, A> {
  inner: C,
  outer: A,
}

impl<C, A> NestedComponent<C, A> {
  pub fn new(inner: C, outer: A) -> Self {
    Self { inner, outer }
  }
}

impl<C, A> Stream for NestedComponent<C, A>
where
  C: Stream<Item = ()> + Unpin,
  A: Stream<Item = ()> + Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut view_changed = false;
    view_changed |= self.outer.poll_next_unpin(cx).is_ready();
    view_changed |= self.inner.poll_next_unpin(cx).is_ready();
    if view_changed {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
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
    self.outer.event(event, &mut self.inner);
  }
}

pub trait PresentableNested<C> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C);
}

impl<C, A: PresentableNested<C>> Presentable for NestedComponent<C, A> {
  fn render(&mut self, builder: &mut crate::PresentationBuilder) {
    self.outer.render(builder, &mut self.inner)
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
    self.outer.layout(constraint, ctx, &mut self.inner)
  }

  fn set_position(&mut self, position: crate::UIPosition) {
    self.outer.set_position(position, &mut self.inner)
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
    self.outer.is_point_in(point, &self.inner)
  }
}
