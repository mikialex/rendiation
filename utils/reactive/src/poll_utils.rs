use crate::*;

// todo, we do not considered poll none case
pub trait PollUtils: Stream + Unpin {
  // synchronously polling the stream, pull out all updates.
  // note, if the compute stream contains async mapping, the async part is actually
  // polled inactively.
  fn loop_poll_until_pending(&mut self, cx: &mut Context, mut on_update: impl FnMut(Self::Item)) {
    while let Poll::Ready(Some(update)) = self.poll_next_unpin(cx) {
      on_update(update)
    }
  }

  fn batch_all_readied(&mut self, cx: &mut Context) -> Vec<Self::Item> {
    let mut results = Vec::new();
    self.loop_poll_until_pending(cx, |r| results.push(r));
    results
  }

  fn count_readied(&mut self, cx: &mut Context) -> usize {
    let mut counter = 0;
    self.loop_poll_until_pending(cx, |_| counter += 1);
    counter
  }

  fn consume_self_get_next(mut self) -> Option<Self::Item>
  where
    Self: Sized,
  {
    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);
    if let Poll::Ready(r) = self.poll_next_unpin(&mut cx) {
      r
    } else {
      None
    }
  }
}

impl<T> PollUtils for T where T: Stream + Unpin {}
