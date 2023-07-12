use std::ops::DerefMut;

use crate::*;

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

/// The helper trait to link different component together
pub trait ComponentNestExt: Sized {
  fn nest_in<A>(self, outer: A) -> NestedComponent<Self, A>
  where
    A: ViewNester<Self>,
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

pub trait ReactiveUpdateNester<C> {
  fn poll_update_inner(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>>;
}

pub trait ViewNester<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C);
}

pub trait HotAreaNester<C> {
  fn is_point_in(&self, _point: crate::UIPosition, _inner: &C) -> bool {
    false
  }
}

impl<C, A> Stream for NestedComponent<C, A>
where
  C: Unpin,
  A: ReactiveUpdateNester<C> + Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    Pin::new(&mut this.outer).poll_update_inner(cx, &mut this.inner)
  }
}

impl<C, A> View for NestedComponent<C, A>
where
  A: ViewNester<C>,
  Self: Stream<Item = ()> + Unpin,
{
  fn request(&mut self, detail: &mut ViewRequest) {
    // the behavior of nested view is fully decided by the nester
    self.outer.request_nester(detail, &mut self.inner)
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

impl<C, A, CC> ReactiveUpdateNester<CC> for NestedComponent<C, A>
where
  Self: Stream<Item = ()> + Unpin,
  CC: Stream<Item = ()> + Unpin,
{
  fn poll_update_inner(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut CC,
  ) -> Poll<Option<()>> {
    // todo, we here to ignore the None case
    let mut r = self.poll_next_unpin(cx).eq(&Poll::Ready(().into()));

    r |= inner.poll_next_unpin(cx).eq(&Poll::Ready(().into()));
    if r {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

impl<C, A, CC> ViewNester<CC> for NestedComponent<C, A>
where
  Self: View,
  CC: View,
{
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut CC) {
    if let ViewRequest::Layout(LayoutProtocol::DoLayout {
      constraint,
      ctx,
      output,
    }) = detail
    {
      let result_self = self.layout(*constraint, ctx);
      let result_inner = self.layout(*constraint, ctx);
      output.baseline_offset = result_inner.baseline_offset; // respect inner?
      output.size = result_self.size.union(result_inner.size)
    } else {
      self.request(detail);
      inner.request(detail);
    }
  }
}

impl<C, A, CC> HotAreaNester<CC> for NestedComponent<C, A>
where
  Self: HotAreaProvider,
  CC: HotAreaProvider,
{
  fn is_point_in(&self, point: crate::UIPosition, inner: &CC) -> bool {
    HotAreaProvider::is_point_in(self, point) || inner.is_point_in(point)
  }
}
