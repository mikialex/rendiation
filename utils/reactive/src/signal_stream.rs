use std::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::{
  ready,
  stream::{once, Fuse, FusedStream},
  Stream, StreamExt,
};
use pin_project::pin_project;

use crate::MergeIntoStreamVec;

pub fn do_updates<T: Stream + Unpin>(stream: &mut T, mut on_update: impl FnMut(T::Item)) {
  // synchronously polling the stream, pull out all updates.
  // note, if the compute stream contains async mapping, the async part is actually
  // polled inactively.
  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  while let Poll::Ready(Some(update)) = stream.poll_next_unpin(&mut cx) {
    on_update(update)
  }
}

pub fn once_forever_pending<T>(value: T) -> impl Stream<Item = T> + Unpin {
  once(core::future::ready(value)).chain(futures::stream::pending())
}

pub trait SignalStreamExt: Stream {
  fn flatten_signal(self) -> FlattenSignalStream<Self, Self::Item>
  where
    Self::Item: Stream,
    Self: Sized;

  fn flatten_into_vec_stream_signal<T>(self) -> MergeIntoStreamVec<Self, T>
  where
    Self: Stream<Item = (usize, Option<T>)>,
    Self: Sized;

  fn zip_signal<St>(self, other: St) -> ZipSignal<Self, St>
  where
    St: Stream,
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

  fn flatten_into_vec_stream_signal<X>(self) -> MergeIntoStreamVec<Self, X>
  where
    Self: Stream<Item = (usize, Option<X>)>,
    Self: Sized,
  {
    MergeIntoStreamVec::new(self)
  }

  fn zip_signal<St>(self, other: St) -> ZipSignal<Self, St>
  where
    St: Stream,
    Self: Sized,
  {
    ZipSignal::new(self, other)
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
      if let Poll::Ready(Some(s)) = this.stream.as_mut().poll_next(cx) {
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

#[pin_project]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ZipSignal<St1: Stream, St2: Stream> {
  #[pin]
  stream1: Fuse<St1>,
  #[pin]
  stream2: Fuse<St2>,
  queued1: Option<St1::Item>,
  queued1_dirty: bool,
  queued2: Option<St2::Item>,
  queued2_dirty: bool,
}

impl<St1: Stream, St2: Stream> ZipSignal<St1, St2> {
  pub(super) fn new(stream1: St1, stream2: St2) -> Self {
    Self {
      stream1: stream1.fuse(),
      stream2: stream2.fuse(),
      queued1: None,
      queued1_dirty: false,
      queued2: None,
      queued2_dirty: false,
    }
  }
}

impl<St1, St2> FusedStream for ZipSignal<St1, St2>
where
  St1: Stream,
  St2: Stream,
  St1::Item: Clone,
  St2::Item: Clone,
{
  fn is_terminated(&self) -> bool {
    self.stream1.is_terminated() && self.stream2.is_terminated()
  }
}

impl<St1, St2> Stream for ZipSignal<St1, St2>
where
  St1: Stream,
  St2: Stream,
  St1::Item: Clone,
  St2::Item: Clone,
{
  type Item = (St1::Item, St2::Item);

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    match this.stream1.as_mut().poll_next(cx) {
      Poll::Ready(Some(item1)) => {
        *this.queued1 = Some(item1);
        *this.queued1_dirty = true;
      }
      Poll::Ready(None) | Poll::Pending => {}
    }

    if this.queued2.is_none() {
      match this.stream2.as_mut().poll_next(cx) {
        Poll::Ready(Some(item2)) => {
          *this.queued2 = Some(item2);
          *this.queued2_dirty = true;
        }
        Poll::Ready(None) | Poll::Pending => {}
      }
    }

    if let (Some(queued1), Some(queued2)) = (this.queued1, this.queued2) {
      if *this.queued1_dirty || *this.queued2_dirty {
        *this.queued1_dirty = false;
        *this.queued2_dirty = false;
        return Poll::Ready(Some((queued1.clone(), queued2.clone())));
      }
    }

    Poll::Pending
  }
}
