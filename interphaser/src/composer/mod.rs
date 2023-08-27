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
pub struct AnyStateHolder<C> {
  inner: C,
  any: Vec<Box<dyn std::any::Any>>,
}

pub trait IntoStateHolder: Sized {
  fn into_state_holder(self) -> AnyStateHolder<Self> {
    AnyStateHolder {
      inner: self,
      any: Default::default(),
    }
  }
}

impl<T> IntoStateHolder for T {}

impl<C> AnyStateHolder<C> {
  pub fn hold_state(mut self, any: impl std::any::Any + 'static) -> Self {
    self.any.push(Box::new(any));
    self
  }
}

impl<CC, C: ViewNester<CC>> ViewNester<CC> for AnyStateHolder<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut CC) {
    self.inner.request_nester(detail, inner)
  }
}

impl<C: View> View for AnyStateHolder<C> {
  fn request(&mut self, detail: &mut ViewRequest) {
    self.inner.request(detail)
  }
}

impl<C: Stream<Item = ()> + Unpin> Stream for AnyStateHolder<C> {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.inner.poll_next_unpin(cx)
  }
}
