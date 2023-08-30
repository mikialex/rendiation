mod nest;
pub use nest::*;

mod array;
pub use array::*;

mod events;
pub use events::*;

mod update;
pub use update::*;

use crate::*;

/// When user use reactive state utils like StateCell, to avoid memory leak by avoiding  any
/// possible circular reference, they always using weak reference and weak check. However, if you
/// using weak every where, where you store the strong reference of your state? The strong reference
/// of the state go with the real ownership of the state, in our ui framework case which is always
/// the view tree. To bind the states lifetime to the view, just push the state or any thing with
/// it.
pub struct AnyHolder<C> {
  inner: C,
  any: Vec<Box<dyn std::any::Any>>,
  // todo improve
  reactive: StreamVec<Box<dyn Stream<Item = ()> + Unpin>>,
  idx: usize,
}

pub trait IntoAnyHolder: Sized {
  fn into_any_holder(self) -> AnyHolder<Self> {
    AnyHolder {
      inner: self,
      any: Default::default(),
      reactive: Default::default(),
      idx: 0,
    }
  }
}

impl<T> IntoAnyHolder for T {}

impl<C> AnyHolder<C> {
  pub fn hold_state(mut self, any: impl std::any::Any + 'static) -> Self {
    self.any.push(Box::new(any));
    self
  }
  pub fn hold_stream(mut self, any: impl Stream<Item = ()> + Unpin + 'static) -> Self {
    self.reactive.insert(self.idx, Some(Box::new(any)));
    self.idx += 1;
    self
  }
}

impl<CC, C: ViewNester<CC>> ViewNester<CC> for AnyHolder<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut CC) {
    self.inner.request_nester(detail, inner)
  }
}

impl<C: View> View for AnyHolder<C> {
  fn request(&mut self, detail: &mut ViewRequest) {
    self.inner.request(detail)
  }
}

impl<C: Stream<Item = ()> + Unpin> Stream for AnyHolder<C> {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let r1 = self.reactive.poll_next_unpin(cx).map(|v| v.map(|_| ()));
    let r2 = self.inner.poll_next_unpin(cx);
    // todo,  move to upstream utils
    match (r1, r2) {
      (Poll::Ready(a), Poll::Ready(b)) => Poll::Ready(a.or(b)),
      (Poll::Ready(v), Poll::Pending) => Poll::Ready(v),
      (Poll::Pending, Poll::Ready(v)) => Poll::Ready(v),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
    }
  }
}
