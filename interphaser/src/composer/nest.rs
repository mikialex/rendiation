use std::ops::DerefMut;

use crate::*;

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

pub trait ViewNester<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C);
}

// impl<C, A: ViewNester<C>> View for NestedComponent<C, A>
// where
//   Self: Unpin,
//   C: Stream,
// {
//   fn request(&mut self, detail: &mut ViewRequest) {
//     self.outer.request_nester(detail, &mut self.inner)
//   }
// }

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
