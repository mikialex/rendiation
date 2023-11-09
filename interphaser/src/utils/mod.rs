mod window_state;
pub use window_state::*;
mod state;
pub use state::*;

use crate::*;

#[macro_export]
macro_rules! trivial_stream_impl {
  ($Type: ty) => {
    impl Stream for $Type {
      type Item = ();
      fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
      }
    }
  };
}

#[derive(Default)]
pub struct ViewUpdateNotifier {
  inner: Option<Waker>,
}

impl ViewUpdateNotifier {
  pub fn notify(&mut self) {
    if let Some(waker) = self.inner.take() {
      waker.wake()
    }
  }
}

impl Stream for ViewUpdateNotifier {
  type Item = ();
  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    if self.inner.is_some() {
      Poll::Pending
    } else {
      self.inner = cx.waker().clone().into();
      Poll::Ready(().into())
    }
  }
}
