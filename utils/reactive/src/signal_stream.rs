use std::collections::VecDeque;

use futures::{
  ready,
  stream::{once, Fuse, FusedStream},
  StreamExt,
};
use pin_project::pin_project;

use crate::*;

pub fn do_updates<T: Stream + Unpin>(stream: &mut T, on_update: impl FnMut(T::Item)) {
  // synchronously polling the stream, pull out all updates.
  // note, if the compute stream contains async mapping, the async part is actually
  // polled inactively.
  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  do_updates_by(stream, &mut cx, on_update)
}

pub fn do_updates_by<T: Stream + Unpin>(
  stream: &mut T,
  cx: &mut Context,
  mut on_update: impl FnMut(T::Item),
) {
  while let Poll::Ready(Some(update)) = stream.poll_next_unpin(cx) {
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

  fn flatten_into_map_stream_signal<T, K>(self) -> MergeIntoStreamMap<Self, K, T>
  where
    Self: Stream<Item = (K, Option<T>)>,
    Self: Sized;

  fn zip_signal<St>(self, other: St) -> ZipSignal<Self, St>
  where
    St: Stream,
    Self: Sized;

  fn filter_map_sync<F>(self, f: F) -> FilterMapSync<Self, F>
  where
    Self: Sized;

  fn buffered_unbound(self) -> BufferedUnbound<Self>
  where
    Self: Sized;

  fn batch_processing(self) -> BatchProcessing<Self>
  where
    Self: Sized;

  fn create_broad_caster(self) -> StreamBroadcaster<Self, Self::Item, FanOut>
  where
    Self: Sized + Stream;

  fn create_index_mapping_broadcaster<D>(self) -> StreamBroadcaster<Self, D, IndexMapping>
  where
    Self: Sized + Stream;

  fn create_batch_index_mapping_broadcaster<X>(self) -> StreamBatchIndexer<Self, X>
  where
    Self: Sized + Stream<Item = Vec<(usize, X)>>;

  fn fold_signal<State, F, X>(self, state: State, f: F) -> SignalFold<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State) -> Option<X>;

  // we elaborate the bound here to help compiler deduce the type
  fn fold_signal_flatten<State, F, X>(
    self,
    state: State,
    f: F,
  ) -> SignalFoldFlatten<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State) -> Option<X>;

  fn fold_signal_state_stream<State, F>(
    self,
    state: State,
    f: F,
  ) -> SignalFoldStateStream<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State);

  fn after_pended_then<S>(self, pending: S) -> StreamDependency<Self, S>
  where
    Self: Sized + Stream;

  fn debug(self, label: impl AsRef<str>) -> DebugStream<Self>
  where
    Self: Sized + Stream;

  fn keep_last(self) -> KeepLast<Self>
  where
    Self: Sized + Stream;
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

  fn flatten_into_map_stream_signal<X, K>(self) -> MergeIntoStreamMap<Self, K, X>
  where
    Self: Stream<Item = (K, Option<X>)>,
    Self: Sized,
  {
    MergeIntoStreamMap::new(self)
  }

  fn zip_signal<St>(self, other: St) -> ZipSignal<Self, St>
  where
    St: Stream,
    Self: Sized,
  {
    ZipSignal::new(self, other)
  }

  fn filter_map_sync<F>(self, f: F) -> FilterMapSync<Self, F>
  where
    Self: Sized,
  {
    FilterMapSync { inner: self, f }
  }

  fn buffered_unbound(self) -> BufferedUnbound<Self> {
    BufferedUnbound {
      inner: self,
      buffered: VecDeque::new(),
    }
  }

  fn batch_processing(self) -> BatchProcessing<Self> {
    BatchProcessing {
      inner: self,
      buffered: Vec::new(),
    }
  }

  fn create_broad_caster(self) -> StreamBroadcaster<Self, Self::Item, FanOut>
  where
    Self: Sized,
  {
    StreamBroadcaster::new(self, FanOut)
  }

  fn create_index_mapping_broadcaster<D>(self) -> StreamBroadcaster<Self, D, IndexMapping>
  where
    Self: Sized,
  {
    StreamBroadcaster::new(self, IndexMapping)
  }

  fn create_batch_index_mapping_broadcaster<X>(self) -> StreamBatchIndexer<Self, X>
  where
    Self: Sized + Stream<Item = Vec<(usize, X)>>,
  {
    StreamBatchIndexer::new(self)
  }

  fn fold_signal<State, F, X>(self, state: State, f: F) -> SignalFold<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State) -> Option<X>,
  {
    SignalFold {
      state,
      stream: self,
      f,
    }
  }

  fn fold_signal_flatten<State, F, X>(self, state: State, f: F) -> SignalFoldFlatten<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State) -> Option<X>,
  {
    SignalFoldFlatten {
      state,
      stream: self,
      f,
    }
  }

  fn fold_signal_state_stream<State, F>(
    self,
    state: State,
    f: F,
  ) -> SignalFoldStateStream<State, Self, F>
  where
    Self: Sized,
    Self: Stream,
    F: FnMut(Self::Item, &mut State),
  {
    SignalFoldStateStream {
      state,
      stream: self,
      f,
    }
  }

  fn after_pended_then<S>(self, pending: S) -> StreamDependency<Self, S>
  where
    Self: Sized + Stream,
  {
    StreamDependency {
      dependency: self,
      source: pending,
    }
  }

  fn debug(self, label: impl AsRef<str>) -> DebugStream<Self>
  where
    Self: Sized + Stream,
  {
    let str = label.as_ref();
    DebugStream {
      inner: self,
      label: str.into(),
    }
  }

  fn keep_last(self) -> KeepLast<Self>
  where
    Self: Sized + Stream,
  {
    KeepLast { inner: self }
  }
}

pub type StreamForker<S> = StreamBroadcaster<S, <S as Stream>::Item, FanOut>;

#[pin_project]
pub struct FilterMapSync<S, F> {
  #[pin]
  inner: S,
  f: F,
}

impl<S, F, X> Stream for FilterMapSync<S, F>
where
  S: Stream,
  F: FnMut(S::Item) -> Option<X>,
{
  type Item = X;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    loop {
      if let Poll::Ready(v) = this.inner.as_mut().poll_next(cx) {
        if let Some(v) = v {
          if let Some(mapped) = (this.f)(v) {
            break Poll::Ready(mapped.into());
          }
        } else {
          break Poll::Ready(None);
        }
      } else {
        break Poll::Pending;
      }
    }
  }
}

#[test]
fn test_filter_map_sync() {
  let (send, rev) = futures::channel::mpsc::unbounded::<u32>();
  send.unbounded_send(10).unwrap();
  send.unbounded_send(3).unwrap(); // will be filtered

  let mut c = rev.filter_map_sync(|v: u32| if v > 5 { Some(2 * v) } else { None });

  do_updates(&mut c, |v| assert_eq!(v, 20))
}

#[pin_project]
pub struct BatchProcessing<S: Stream> {
  #[pin]
  inner: S,
  buffered: Vec<S::Item>,
}

impl<S> Stream for BatchProcessing<S>
where
  S: Stream,
{
  type Item = Vec<S::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    while let Poll::Ready(result) = this.inner.as_mut().poll_next(cx) {
      if let Some(item) = result {
        this.buffered.push(item);
        continue;
      } else {
        return Poll::Ready(None); // the source has been dropped, do early terminate
      }
    }

    if this.buffered.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(std::mem::take(this.buffered)))
    }
  }
}

#[pin_project]
pub struct BufferedUnbound<S: Stream> {
  #[pin]
  inner: S,
  buffered: VecDeque<S::Item>,
}

impl<S> Stream for BufferedUnbound<S>
where
  S: Stream,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    while let Poll::Ready(result) = this.inner.as_mut().poll_next(cx) {
      if let Some(item) = result {
        this.buffered.push_back(item);
        continue;
      } else {
        return Poll::Ready(None); // the source has been dropped, do early terminate
      }
    }

    if let Some(item) = this.buffered.pop_front() {
      Poll::Ready(Some(item))
    } else {
      Poll::Pending
    }
  }
}

#[test]
fn test_buffer_unbound() {
  let (send, rev) = futures::channel::mpsc::unbounded::<u32>();

  let mut front = 0;
  let mut back = 0;

  let mut c = rev
    .map(|v| {
      front += 1;
      v
    })
    .buffered_unbound()
    .map(|v| {
      back += 1;
      v
    });

  send.unbounded_send(10).unwrap();
  send.unbounded_send(3).unwrap();
  send.unbounded_send(31).unwrap();

  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  let r = c.poll_next_unpin(&mut cx);

  assert_eq!(r, Poll::Ready(Some(10)));
  assert_eq!(front, 3);
  assert_eq!(back, 1);
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

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
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

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    match this.stream1.as_mut().poll_next(cx) {
      Poll::Ready(Some(item1)) => {
        *this.queued1 = Some(item1);
        *this.queued1_dirty = true;
      }
      Poll::Ready(None) | Poll::Pending => {}
    }

    match this.stream2.as_mut().poll_next(cx) {
      Poll::Ready(Some(item2)) => {
        *this.queued2 = Some(item2);
        *this.queued2_dirty = true;
      }
      Poll::Ready(None) | Poll::Pending => {}
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

#[pin_project]
pub struct SignalFold<T, S, F> {
  state: T,
  #[pin]
  stream: S,
  f: F,
}

impl<T, S, F> AsRef<T> for SignalFold<T, S, F> {
  fn as_ref(&self) -> &T {
    &self.state
  }
}

impl<T, S, F> AsMut<T> for SignalFold<T, S, F> {
  fn as_mut(&mut self) -> &mut T {
    &mut self.state
  }
}

impl<T, S, F, X> Stream for SignalFold<T, S, F>
where
  S: Stream,
  F: FnMut(S::Item, &mut T) -> Option<X>,
{
  type Item = X;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    loop {
      if let Poll::Ready(v) = this.stream.as_mut().poll_next(cx) {
        if let Some(v) = v {
          if let Some(c) = (this.f)(v, this.state) {
            break Poll::Ready(Some(c));
          }
        } else {
          break Poll::Ready(None);
        }
      } else {
        break Poll::Pending;
      }
    }
  }
}

#[pin_project]
/// we could use Arc state and stream select to achieve same effect
pub struct SignalFoldFlatten<T, S, F> {
  state: T,
  #[pin]
  stream: S,
  f: F,
}

impl<T, S, F> AsRef<T> for SignalFoldFlatten<T, S, F> {
  fn as_ref(&self) -> &T {
    &self.state
  }
}

impl<T, S, F, X> Stream for SignalFoldFlatten<T, S, F>
where
  S: Stream,
  T: Stream<Item = X> + Unpin,
  F: FnMut(S::Item, &mut T) -> Option<X>,
{
  type Item = X;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    loop {
      if let Poll::Ready(v) = this.stream.as_mut().poll_next(cx) {
        if let Some(v) = v {
          if let Some(v) = (this.f)(v, this.state) {
            break Poll::Ready(Some(v));
          }
        } else {
          break Poll::Ready(None);
        }
      } else {
        break this.state.poll_next_unpin(cx);
      }
    }
  }
}

#[pin_project]
pub struct SignalFoldStateStream<T, S, F> {
  #[pin]
  state: T,
  #[pin]
  stream: S,
  f: F,
}

impl<T, S, F> AsRef<T> for SignalFoldStateStream<T, S, F> {
  fn as_ref(&self) -> &T {
    &self.state
  }
}

impl<T, S, X, F> Stream for SignalFoldStateStream<T, S, F>
where
  S: Stream,
  T: Stream<Item = X> + Unpin,
  F: FnMut(S::Item, &mut T),
{
  type Item = X;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    while let Poll::Ready(v) = this.stream.as_mut().poll_next(cx) {
      if let Some(v) = v {
        (this.f)(v, &mut this.state);
        continue;
      } else {
        return Poll::Ready(None);
      }
    }
    this.state.poll_next(cx)
  }
}

/// make sure when poll source, the dependency will in pending state
#[pin_project::pin_project]
pub struct StreamDependency<D, T> {
  #[pin]
  dependency: D,
  #[pin]
  source: T,
}

impl<D, T> Stream for StreamDependency<D, T>
where
  D: Stream<Item = ()>,
  T: Stream,
{
  type Item = T::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    while let Poll::Ready(v) = this.dependency.as_mut().poll_next(cx) {
      if v.is_none() {
        return Poll::Ready(None);
      }
    }

    this.source.poll_next(cx)
  }
}

#[pin_project::pin_project]
pub struct DebugStream<S> {
  #[pin]
  inner: S,
  label: String,
}

impl<S> Stream for DebugStream<S>
where
  S: Stream,
  S::Item: std::fmt::Debug,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_next(cx);
    if let Poll::Ready(r) = &r {
      if let Some(r) = r {
        println!("stream <{}> receive message: {:?}", this.label, r)
      } else {
        println!("stream <{}> terminate", this.label)
      }
    }

    r
  }
}

#[pin_project::pin_project]
pub struct KeepLast<S> {
  #[pin]
  inner: S,
}

impl<S> Stream for KeepLast<S>
where
  S: Stream,
  S::Item: std::fmt::Debug,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    let mut has_ready = false;
    let mut last = None;
    while let Poll::Ready(r) = this.inner.as_mut().poll_next(cx) {
      has_ready = true;
      last = r;
    }

    if has_ready {
      Poll::Ready(last)
    } else {
      Poll::Pending
    }
  }
}
