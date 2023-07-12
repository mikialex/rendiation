pub mod window_state;
pub use window_state::*;
pub mod state;
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

#[macro_export]
macro_rules! trivial_stream_nester_impl {
  ($Type: ty) => {
    impl<C: Stream<Item = ()> + Unpin> ReactiveUpdateNester<C> for $Type {
      fn poll_update_inner(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        inner: &mut C,
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
