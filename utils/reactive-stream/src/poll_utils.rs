use crate::*;

/// note, this is just for convenience purpose, please use poll utils
pub fn do_updates<T: Stream + Unpin>(stream: &mut T, mut on_update: impl FnMut(T::Item)) {
  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  while let Poll::Ready(Some(update)) = stream.poll_next_unpin(&mut cx) {
    on_update(update)
  }
}

pub trait PollUtils: Stream + Unpin {
  // pull out all updates.
  fn poll_until_pending(&mut self, cx: &mut Context, mut on_update: impl FnMut(Self::Item))
  where
    Self: FusedStream,
  {
    while let Poll::Ready(Some(update)) = self.poll_next_unpin(cx) {
      on_update(update)
    }
  }

  // pull out all updates. return if trigger terminate
  #[must_use]
  fn poll_until_pending_or_terminate(
    &mut self,
    cx: &mut Context,
    mut on_update: impl FnMut(Self::Item),
  ) -> bool {
    while let Poll::Ready(r) = self.poll_next_unpin(cx) {
      if let Some(update) = r {
        on_update(update)
      } else {
        return true;
      }
    }
    false
  }

  // pull out all updates.
  fn poll_until_pending_not_care_result(&mut self, cx: &mut Context)
  where
    Self: FusedStream,
  {
    self.poll_until_pending(cx, |_| {})
  }

  // pull out all updates.
  #[must_use]
  fn poll_until_pending_or_terminate_not_care_result(&mut self, cx: &mut Context) -> bool {
    self.poll_until_pending_or_terminate(cx, |_| {})
  }

  fn batch_all_readied(&mut self, cx: &mut Context) -> Vec<Self::Item>
  where
    Self: FusedStream,
  {
    let mut results = Vec::new();
    self.poll_until_pending(cx, |r| results.push(r));
    results
  }

  fn count_readied(&mut self, cx: &mut Context) -> usize
  where
    Self: FusedStream,
  {
    let mut counter = 0;
    self.poll_until_pending(cx, |_| counter += 1);
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

#[pin_project::pin_project]
pub struct DropperAttachedStream<T, S> {
  dropper: T,
  #[pin]
  stream: S,
}

impl<T, S> DropperAttachedStream<T, S> {
  pub fn new(dropper: T, stream: S) -> Self {
    Self { dropper, stream }
  }
}

impl<T, S> Stream for DropperAttachedStream<T, S>
where
  S: Stream,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.project().stream.poll_next(cx)
  }
}
