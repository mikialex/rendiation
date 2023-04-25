use std::collections::HashMap;

use futures::{stream::FuturesUnordered, *};

use crate::*;

pub trait ReactiveMapping<M> {
  type ChangeStream: Stream + Unpin;
  type DropFuture: Future<Output = ()> + Unpin;
  type Ctx<'a>;

  fn key(&self) -> usize;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream, Self::DropFuture);

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>);
}

pub struct ReactiveMap<T: ReactiveMapping<M>, M> {
  mapping: HashMap<usize, (M, T::ChangeStream)>,
  /// when drop consumed, we remove the mapped from mapping, we could make this sync to drop.
  /// but if we do so, the mapping have to wrapped in interior mutable container, and it's
  /// impossible to get mut reference directly in safe rust.
  ///
  /// user should call cleanup periodically to do the actually remove now.
  drop_futures: FuturesUnordered<KeyedDropFuture<T::DropFuture, usize>>,
}

impl<M, T: ReactiveMapping<M>> Default for ReactiveMap<T, M> {
  fn default() -> Self {
    Self {
      mapping: Default::default(),
      drop_futures: Default::default(),
    }
  }
}

type KeyedDropFuture<F: Future<Output = ()>, T> = impl Future<Output = T>;
fn map_drop_future<T, F: Future<Output = ()>>(f: F, key: T) -> KeyedDropFuture<F, T> {
  f.map(|_| key)
}

impl<M, T: ReactiveMapping<M>> ReactiveMap<T, M> {
  pub fn get_with_update(&mut self, source: &T, ctx: &T::Ctx<'_>) -> &mut M {
    self.cleanup();

    let id = T::key(source);

    let (mapped, changes) = self.mapping.entry(id).or_insert_with(|| {
      let (mapped, stream, future) = T::build(source, ctx);
      self.drop_futures.push(map_drop_future(future, id));
      (mapped, stream)
    });

    source.update(mapped, changes, ctx);
    mapped
  }

  pub fn cleanup(&mut self) {
    do_updates(&mut self.drop_futures, |id| {
      self.mapping.remove(&id);
    })
  }
}

#[pin_project::pin_project]
pub struct StreamMap<T> {
  streams: HashMap<usize, T>,
  waked: Arc<RwLock<Vec<usize>>>,
  waker: Arc<RwLock<Option<Waker>>>,
}

impl<T> Default for StreamMap<T> {
  fn default() -> Self {
    Self {
      streams: Default::default(),
      waked: Default::default(),
      waker: Default::default(),
    }
  }
}

fn try_wake(w: &Arc<RwLock<Option<Waker>>>) {
  let waker = w.read().unwrap();
  let waker: &Option<_> = &waker;
  if let Some(waker) = waker {
    waker.wake_by_ref();
  }
}

impl<T> StreamMap<T> {
  pub fn get(&self, key: usize) -> Option<&T> {
    self.streams.get(&key)
  }

  pub fn get_or_insert_with(&mut self, key: usize, f: impl FnOnce() -> T) -> &mut T {
    self.streams.entry(key).or_insert_with(|| {
      self.waked.write().unwrap().push(key);
      try_wake(&self.waker);
      f()
    })
  }

  pub fn remove(&mut self, key: usize) {
    self.streams.remove(&key);
  }

  pub fn try_wake(&self) {
    try_wake(&self.waker)
  }
}

impl<T: Stream + Unpin> Stream for StreamMap<T> {
  type Item = IndexedItem<T::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<Option<Self::Item>> {
    let this = self.project();
    let mut changed = this.waked.write().unwrap();

    this.waker.write().unwrap().replace(cx.waker().clone());

    while let Some(&index) = changed.last() {
      let waker = Arc::new(ChangeWaker {
        waker: this.waker.clone(),
        index,
        changed: this.waked.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx = Context::from_waker(&waker);

      if let Some(stream) = this.streams.get_mut(&index) {
        if let Poll::Ready(r) = stream
          .poll_next_unpin(&mut cx)
          .map(|r| r.map(|item| IndexedItem { index, item }))
        {
          if r.is_none() {
            this.streams.remove(&index);
          } else {
            return Poll::Ready(r);
          }
        }
      }

      changed.pop().unwrap();
    }
    Poll::Pending
  }
}
