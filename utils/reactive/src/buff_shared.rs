use std::{
  collections::VecDeque,
  pin::Pin,
  sync::{atomic::AtomicU64, Arc, RwLock},
  task::Poll,
};

use arena::{Arena, Handle};
use futures::Stream;
use pin_project::pin_project;

#[pin_project]
struct SharedBufferedStreamInner<S: Stream> {
  #[pin]
  inner: S,
  buffered: VecDeque<S::Item>,
  latest_buffered_generation: u64,
  consumers_generation: Arena<Arc<AtomicU64>>,
}

struct SharedBufferedStream<S: Stream> {
  inner: Arc<RwLock<SharedBufferedStreamInner<S>>>,
  cursor: Arc<AtomicU64>,
  index: Handle<Arc<AtomicU64>>,
}

impl<S> Stream for SharedBufferedStream<S>
where
  S: Stream + Unpin,
  S::Item: Clone,
{
  type Item = S::Item;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write().unwrap();
    let inner: &mut SharedBufferedStreamInner<_> = &mut inner;
    let inner = Pin::new(inner);
    let this = inner.project();
    if let Poll::Ready(v) = this.inner.poll_next(cx) {
      if let Some(v) = v {
        this.buffered.push_back(v);
        *this.latest_buffered_generation += 1;
      } else {
        // early forward termination
        return Poll::Ready(None);
      }
    }

    // get buffered ready result
    let ready = if let Some(v) = this.buffered.get(
      self
        .cursor
        .fetch_add(0, std::sync::atomic::Ordering::SeqCst) as usize,
    ) {
      self
        .cursor
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      Poll::Ready(Some(v.clone()))
    } else {
      // the buffered is empty
      return Poll::Pending;
    };

    // cleanup buffer history
    // todo: performance is super bad!
    let earliest = this
      .consumers_generation
      .iter()
      .map(|(_, v)| v.fetch_add(0, std::sync::atomic::Ordering::SeqCst))
      .min()
      .unwrap();
    let real_size = *this.latest_buffered_generation - earliest + 1;
    while this.buffered.len() > real_size as usize {
      this.buffered.pop_front();
    }

    ready
  }
}

impl<S: Stream> Drop for SharedBufferedStream<S> {
  fn drop(&mut self) {
    self
      .inner
      .write()
      .unwrap()
      .consumers_generation
      .remove(self.index);
  }
}

impl<S: Stream> Clone for SharedBufferedStream<S> {
  fn clone(&self) -> Self {
    let cursor = Arc::new(AtomicU64::new(
      self
        .cursor
        .fetch_add(0, std::sync::atomic::Ordering::SeqCst),
    ));

    let mut inner = self.inner.write().unwrap();
    let index = inner.consumers_generation.insert(cursor.clone());

    Self {
      inner: self.inner.clone(),
      cursor,
      index,
    }
  }
}
