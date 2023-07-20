use std::hash::Hash;

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
  mapping: FastHashMap<usize, (M, T::ChangeStream)>,
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
  streams: FastHashMap<K, T>,
  ref_changes: Vec<RefChange<K>>,
  waked: Arc<SegQueue<K>>,
  waker: Arc<AtomicWaker>,
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

impl<K: Hash + Eq + Clone, T> StreamMap<K, T> {
  pub fn get(&self, key: &K) -> Option<&T> {
    self.streams.get(key)
  }
  pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
    self.streams.get_mut(key)
  }
  pub fn len(&self) -> usize {
    self.streams.len()
  }
  pub fn is_empty(&self) -> bool {
    self.streams.is_empty()
  }
  pub fn values(&self) -> impl Iterator<Item = &T> {
    self.streams.values()
  }

  pub fn insert(&mut self, key: K, value: T) {
    // handle replace semantic
    if self.streams.contains_key(&key) {
      self.ref_changes.push(RefChange::Remove(key.clone()));
    }
    self.streams.insert(key.clone(), value);
    self.waked.push(key.clone());
    self.ref_changes.push(RefChange::Insert(key));
    self.waker.wake()
  }

  pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> T) -> &mut T {
    self.streams.entry(key.clone()).or_insert_with(|| {
      self.waked.push(key.clone());
      self.ref_changes.push(RefChange::Insert(key));
      self.waker.wake();
      f()
    })
  }

  pub fn remove(&mut self, key: K) -> Option<T> {
    self.waker.wake();
    self.streams.remove(&key).map(|d| {
      self.ref_changes.push(RefChange::Remove(key));
      d
    })
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

impl<K, T> StreamMapDelta<K, T> {
  pub fn map<U>(self, f: impl FnOnce(&K, T) -> U) -> StreamMapDelta<K, U> {
    match self {
      StreamMapDelta::Insert(k) => StreamMapDelta::Insert(k),
      StreamMapDelta::Remove(k) => StreamMapDelta::Remove(k),
      StreamMapDelta::Delta(k, v) => {
        let v = f(&k, v);
        StreamMapDelta::Delta(k, v)
      }
    }
  }
}

impl<K, T> FusedStream for StreamMap<K, T>
where
  Self: Stream,
{
  fn is_terminated(&self) -> bool {
    false // reactive container never terminates
  }
}

impl<K, T> Stream for StreamMap<K, T>
where
  K: Clone + Send + Sync + Hash + Eq,
  T: Stream + Unpin,
{
  // we use the batched message to optimize the performance
  type Item = Vec<StreamMapDelta<K, T::Item>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    // install new waker
    this.waker.register(cx.waker());

    // note: this is not precise estimation, because each waked value maybe emit multiple delta
    let waked_size = this.waked.len();
    let result_size = this.ref_changes.len() + waked_size;
    if result_size == 0 {
      return Poll::Pending;
    }
    let mut results = Vec::with_capacity(result_size);

    while let Some(change) = this.ref_changes.pop() {
      let d = match change {
        RefChange::Insert(d) => StreamMapDelta::Insert(d),
        RefChange::Remove(d) => StreamMapDelta::Remove(d),
      };
      results.push(d)
    }

    loop {
      let last = this.waked.pop();
      if let Some(key) = last {
        // prepare the sub waker
        let waker = Arc::new(ChangeWaker {
          waker: this.waker.clone(),
          index: key.clone(),
          changed: this.waked.clone(),
        });
        let waker = futures::task::waker_ref(&waker);
        let mut cx = Context::from_waker(&waker);

        // poll the sub stream
        if let Some(stream) = this.streams.get_mut(&key) {
          while let Poll::Ready(r) = stream.poll_next_unpin(&mut cx) {
            if let Some(r) = r {
              results.push(StreamMapDelta::Delta(key.clone(), r));
            } else {
              this.streams.remove(&key);
              results.push(StreamMapDelta::Remove(key.clone()));
              break;
            }
          }
        }
      } else {
        break;
      }
    }

    // even sub stream waked, they maybe not poll any message out
    if results.is_empty() {
      return Poll::Pending;
    }

    Poll::Ready(results.into())
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
  type Item = Vec<StreamMapDelta<K, T::Item>>;

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
