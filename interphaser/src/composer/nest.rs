use std::ops::DerefMut;

use crate::*;

/// Combinator structure, to combine a parent view with a single child view
pub struct NestedView<C, A> {
  pub inner: C,
  pub nester: A,
}

impl<C, A> NestedView<C, A> {
  pub fn new(inner: C, nester: A) -> Self {
    Self { inner, nester }
  }
}

/// The helper trait to link different component together
pub trait ViewNestExt: Sized {
  fn nest_in<A>(self, nester: A) -> NestedView<Self, A>
  where
    A: ViewNester<Self>,
  {
    NestedView::new(self, nester)
  }
  fn wrap<C>(self, inner: C) -> NestedView<C, Self>
where
    // Self: ComponentNester<C>, 
    // todo check if compiler bug?
  {
    NestedView::new(inner, self)
  }
}
impl<X> ViewNestExt for X where X: Sized {}

pub trait ViewNester<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C);
}

impl<C, A> Stream for NestedView<C, A>
where
  C: Stream<Item = ()> + Unpin,
  A: Stream<Item = ()> + Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    // todo, we here to ignore the None case
    let mut r = this.inner.poll_next_unpin(cx).eq(&Poll::Ready(().into()));
    r |= this.nester.poll_next_unpin(cx).eq(&Poll::Ready(().into()));
    if r {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

impl<C, A> View for NestedView<C, A>
where
  A: ViewNester<C>,
  Self: Stream<Item = ()> + Unpin,
{
  fn request(&mut self, detail: &mut ViewRequest) {
    // the behavior of nested view is fully decided by the nester
    self.nester.request_nester(detail, &mut self.inner)
  }
}

// if nested view work as the nester, the nester behavior if fully decided by the nested
// and what important, the nest it self should impl view. self view update happens before the nest
// logic
impl<A, C: ViewNester<CC>, CC> ViewNester<CC> for NestedView<C, A>
where
  Self: View,
{
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut CC) {
    self.request(detail);
    self.inner.request_nester(detail, inner)
  }
}
