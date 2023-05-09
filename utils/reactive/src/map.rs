use std::{collections::HashMap, hash::Hash};

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
pub struct StreamMap<K, T> {
  streams: HashMap<K, T>,
  ref_changes: Vec<RefChange<K>>,
  waked: Arc<RwLock<Vec<K>>>,
  waker: Arc<RwLock<Option<Waker>>>,
}

impl<K, T> Default for StreamMap<K, T> {
  fn default() -> Self {
    Self {
      streams: Default::default(),
      ref_changes: Default::default(),
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

impl<K: Hash + Eq + Clone, T> StreamMap<K, T> {
  pub fn get(&self, key: &K) -> Option<&T> {
    self.streams.get(key)
  }
  pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
    self.streams.get_mut(key)
  }

  pub fn insert(&mut self, key: K, value: T) {
    // handle replace semantic
    if self.streams.contains_key(&key) {
      self.ref_changes.push(RefChange::Remove(key.clone()));
    }
    self.streams.insert(key.clone(), value);
    self.waked.write().unwrap().push(key.clone());
    self.ref_changes.push(RefChange::Insert(key));
    self.try_wake()
  }

  pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> T) -> &mut T {
    self.streams.entry(key.clone()).or_insert_with(|| {
      self.waked.write().unwrap().push(key.clone());
      self.ref_changes.push(RefChange::Insert(key));
      try_wake(&self.waker);
      f()
    })
  }

  pub fn remove(&mut self, key: K) -> Option<T> {
    self.streams.remove(&key).map(|d| {
      self.ref_changes.push(RefChange::Remove(key));
      d
    })
  }

  pub fn try_wake(&self) {
    try_wake(&self.waker)
  }
}

enum RefChange<K> {
  Insert(K),
  Remove(K),
}

pub enum StreamMapDelta<K, T> {
  Insert(K),
  Remove(K),
  Delta(K, T),
}

impl<K, T> Stream for StreamMap<K, T>
where
  K: Clone + Send + Sync + Hash + Eq,
  T: Stream + Unpin,
{
  type Item = StreamMapDelta<K, T::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();

    if let Some(change) = this.ref_changes.pop() {
      let d = match change {
        RefChange::Insert(d) => StreamMapDelta::Insert(d),
        RefChange::Remove(d) => StreamMapDelta::Remove(d),
      };
      return Poll::Ready(d.into());
    }

    this.waker.write().unwrap().replace(cx.waker().clone());

    loop {
      let last = this.waked.read().unwrap().last().cloned();
      if let Some(index) = last {
        let waker = Arc::new(ChangeWaker {
          waker: this.waker.clone(),
          index: index.clone(),
          changed: this.waked.clone(),
        });
        let waker = futures::task::waker_ref(&waker);
        let mut cx = Context::from_waker(&waker);

        if let Some(stream) = this.streams.get_mut(&index) {
          if let Poll::Ready(r) = stream.poll_next_unpin(&mut cx) {
            if let Some(r) = r {
              return Poll::Ready(StreamMapDelta::Delta(index, r).into());
            } else {
              this.streams.remove(&index);
              return Poll::Ready(StreamMapDelta::Remove(index).into());
            }
          }
        }

        this.waked.write().unwrap().pop().unwrap();
      } else {
        break;
      }
    }

    Poll::Pending
  }
}

#[pin_project]
pub struct MergeIntoStreamMap<S, K, T> {
  #[pin]
  inner: S,
  #[pin]
  map: StreamMap<K, T>,
}

impl<S, K, T> AsRef<StreamMap<K, T>> for MergeIntoStreamMap<S, K, T> {
  fn as_ref(&self) -> &StreamMap<K, T> {
    &self.map
  }
}

impl<S, K, T> AsMut<StreamMap<K, T>> for MergeIntoStreamMap<S, K, T> {
  fn as_mut(&mut self) -> &mut StreamMap<K, T> {
    &mut self.map
  }
}

impl<S, K, T> MergeIntoStreamMap<S, K, T> {
  pub fn new(inner: S) -> Self {
    Self {
      inner,
      map: Default::default(),
    }
  }
}

impl<S, K, T> Stream for MergeIntoStreamMap<S, K, T>
where
  S: Stream<Item = (K, Option<T>)>,
  T: Stream + Unpin,
  K: Clone + Send + Sync + Hash + Eq,
{
  type Item = StreamMapDelta<K, T::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    if let Poll::Ready(next) = this.inner.poll_next(cx) {
      if let Some((index, result)) = next {
        if let Some(result) = result {
          this.map.insert(index, result);
        } else {
          this.map.remove(index);
        }
      } else {
        return Poll::Ready(None);
      }
    }

    // the vec will never terminated
    if let Poll::Ready(Some(d)) = this.map.poll_next(cx) {
      return Poll::Ready(Some(d));
    }

    Poll::Pending
  }
}
