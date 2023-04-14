use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
  task::Waker,
};

use futures::{stream::FuturesUnordered, *};

use crate::do_updates;

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

pub struct StreamMap<T> {
  contents: HashMap<usize, T>,
  waked: Arc<RwLock<Vec<usize>>>,
  waker: Arc<RwLock<Option<Waker>>>,
}

fn try_wake(w: &Arc<RwLock<Option<Waker>>>) {
  let waker = w.read().unwrap();
  let waker: &Option<_> = &waker;
  if let Some(waker) = waker {
    waker.wake_by_ref();
  }
}

impl<T> StreamMap<T> {
  pub fn get_or_insert_with(&mut self, key: usize, f: impl FnOnce() -> T) -> &mut T {
    self.contents.entry(key).or_insert_with(|| {
      self.waked.write().unwrap().push(key);
      try_wake(&self.waker);
      f()
    })
  }

  pub fn try_wake(&self) {
    try_wake(&self.waker)
  }
}

impl<T: Stream + Unpin> Stream for StreamMap<T> {
  type Item = T::Item;

  fn poll_next(
    self: core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    todo!()
  }
  //
}
