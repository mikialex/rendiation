use std::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::{ready, stream::FusedStream, Stream, StreamExt};
use pin_project::pin_project;

pub fn do_updates<T: Stream + Unpin>(stream: &mut T, mut on_update: impl FnMut(T::Item)) {
  // synchronously polling the stream, pull all updates out.
  // note, if the compute stream contains async mapping, the async part is actually
  // polled inactively.
  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  while let Poll::Ready(Some(update)) = stream.poll_next_unpin(&mut cx) {
    on_update(update)
  }
}

pub trait SignalStreamExt: Stream {
  fn flatten_signal(self) -> FlattenSignalStream<Self, Self::Item>
  where
    Self::Item: Stream,
    Self: Sized;
}

impl<T: Stream> SignalStreamExt for T {
  fn flatten_signal(self) -> FlattenSignalStream<Self, Self::Item>
  where
    Self::Item: Stream,
    Self: Sized,
  {
    FlattenSignalStream::new(self)
  }
}

#[pin_project]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct FlattenSignalStream<St, U> {
  #[pin]
  stream: St,
  #[pin]
  next: Option<U>,
}

impl<St, U> FlattenSignalStream<St, U> {
  pub(super) fn new(stream: St) -> Self {
    Self { stream, next: None }
  }

  //   delegate_access_inner!(stream, St, ());
}

impl<St> FusedStream for FlattenSignalStream<St, St::Item>
where
  St: FusedStream,
  St::Item: Stream,
{
  fn is_terminated(&self) -> bool {
    self.next.is_none() && self.stream.is_terminated()
  }
}

impl<St> Stream for FlattenSignalStream<St, St::Item>
where
  St: Stream,
  St::Item: Stream,
{
  type Item = <St::Item as Stream>::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    Poll::Ready(loop {
      // compare to the flatten, we poll the outside stream first
      if let Some(s) = ready!(this.stream.as_mut().poll_next(cx)) {
        this.next.set(Some(s));
      } else if let Some(s) = this.next.as_mut().as_pin_mut() {
        if let Some(item) = ready!(s.poll_next(cx)) {
          break Some(item);
        } else {
          this.next.set(None);
        }
      } else {
        break None;
      }
    })
  }
}
