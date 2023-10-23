use std::marker::PhantomData;

use fast_hash_collection::FastHashMap;

use crate::*;

// post/async transform
// Vec<KVDelta<K, V>> ==(if V::Delta==V, single value)=> Vec<KVDelta<K, V>>
// Vec<KVDelta<K, V>> ==(drop any invalid in history + group by k)=> Vec<KVDelta<K, V>>

// sync reduce
// group single value
// group multi value

pub enum VirtualKVCollectionDelta<K, V> {
  /// here we not impose any delta on
  Delta(K, V),
  Remove(K),
}

impl<K, V> VirtualKVCollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl FnOnce(V) -> R) -> VirtualKVCollectionDelta<K, R> {
    type Rt<K, R> = VirtualKVCollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => Rt::<K, R>::Delta(k, mapper(d)),
    }
  }

  // should we just use struct??
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k) => k,
      Self::Delta(k, _) => k,
    }
  }
}

pub trait VirtualKVCollection<K, V> {
  // fn iter(&self) -> impl Iterator<Item = (K, V)>;

  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V>));
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine this is a data table with the K as the primary key and V as the data table.
/// In this table, besides getting data, you can also poll it's partial change.
///
/// This trait maybe could generalize to SignalLike trait:
/// ```rust
/// pub trait Signal<T: IncrementalBase>: Stream<Item = T::Delta> {
///   fn access(&self) -> T;
/// }
/// ```
/// However, this idea has is not baked enough. For example, how do we express efficient partial
/// access for large T or container like T? Should we use some accessor associate trait or type as
/// the accessor key? Should we link this type to the T like how we did in Incremental trait?
pub trait ReactiveKVCollection<K, V>: VirtualKVCollection<K, V> + Stream + Unpin
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

/// The data maybe slate if we combine these two trait directly because the visitor maybe not
/// directly access the original source data, but access the cache. This access abstract the
/// internal cache mechanism. Note, even if the polling issued before access, you still can not
/// guaranteed to access the "current" data due to the multi-threaded source mutation. Because of
/// this limitation, user should make sure their downstream consuming logic is timeline insensitive.
///
/// In the future, maybe we could add new sub-trait to enforce the data access is consistent with
/// the polling logic in tradeoff of the potential memory overhead.
impl<T, K, V> ReactiveKVCollection<K, V> for T
where
  T: VirtualKVCollection<K, V> + Stream + Unpin,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

pub trait ReactiveKVCollectionExt<K, V>: Sized + 'static + ReactiveKVCollection<K, V>
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  /// map map<k, v> to map<k, v2>
  fn map<V2, F: Fn(V) -> V2 + Copy>(self, f: F) -> ReactiveKVMap<Self, F, K, V2> {
    ReactiveKVMap {
      inner: self,
      map: f,
      k: PhantomData,
      pre_v: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn filter<F>(self, f: F) -> ReactiveKVFilter<Self, F, K> {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  /// filter map<k, v> by reactive set<k>
  // fn filter_by_keyset(self, set:)

  // fn zip<V2>(
  //   self,
  //   other: impl ReactiveKVCollection<K, V2>,
  // ) -> impl ReactiveKVCollection<K, (V, V2)> {
  //   //
  // }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveKVMap<Self, K, V> {
    UnorderedMaterializedReactiveKVMap {
      inner: self,
      cache: Default::default(),
    }
  }
}
impl<T, K, V> ReactiveKVCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveKVCollection<K, V>,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

#[pin_project::pin_project]
pub struct UnorderedMaterializedReactiveKVMap<Map, K, V> {
  #[pin]
  inner: Map,
  cache: FastHashMap<K, V>,
}

impl<Map, K, V> Stream for UnorderedMaterializedReactiveKVMap<Map, K, V>
where
  Map: Stream,
  K: std::hash::Hash + Eq,
  V: IncrementalBase<Delta = V>,
  Map::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>> + Clone,
{
  type Item = Map::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_next(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          VirtualKVCollectionDelta::Delta(k, v) => {
            this.cache.insert(k, v);
          }
          VirtualKVCollectionDelta::Remove(k) => {
            // todo, shrink
            this.cache.remove(&k);
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualKVCollection<K, V> for UnorderedMaterializedReactiveKVMap<Map, K, V>
where
  Map: VirtualKVCollection<K, V>,
  K: std::hash::Hash + Eq,
  V: Clone,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V>)) {
    getter(&|key| self.cache.get(&key).cloned())
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  k: PhantomData<K>,
  pre_v: PhantomData<V>,
}

impl<T, F, K, V, V2> Stream for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mapper = *this.map;
    this
      .inner
      .poll_next(cx)
      .map(move |r| r.map(move |deltas| deltas.into_iter().map(move |delta| delta.map(mapper))))
  }
}

impl<T, F, K, V, V2> VirtualKVCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V2>)) {
    self
      .inner
      .access(move |inner_getter| getter(&|key| inner_getter(key).map(|v| (self.map)(v))))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVFilter<T, F, K> {
  #[pin]
  inner: T,
  checker: F,
  k: PhantomData<K>,
}

impl<T, F, K, V> Stream for ReactiveKVFilter<T, F, K>
where
  F: Fn(&V) -> bool + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let checker = *this.checker;
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas.into_iter().filter(move |delta| match delta {
          VirtualKVCollectionDelta::Delta(_, v) => checker(v),
          // the Remove variant maybe called many times for given k
          VirtualKVCollectionDelta::Remove(_) => true,
        })
      })
    })
  }
}

impl<T, F, K, V> VirtualKVCollection<K, V> for ReactiveKVFilter<T, F, K>
where
  F: Fn(&V) -> bool + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V>)) {
    self.inner.access(move |inner_getter| {
      getter(&|key| inner_getter(key).and_then(|v| (self.checker)(&v).then_some(v)))
    })
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVZip<T1, T2, K> {
  #[pin]
  a: T1,
  #[pin]
  b: T2,
  k: PhantomData<K>,
}

impl<T1, T2, K, V1, V2> Stream for ReactiveKVZip<T1, T2, K>
where
  T1: Stream,
  T1::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V1>>,
  T2: Stream,
  T2::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, (V1, V2)>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();

    let t1 = this.a.poll_next(cx);
    let t2 = this.b.poll_next(cx);

    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        // let intersections = FastHashMap::default();
        Poll::Ready(Some(Vec::new()))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(Vec::new())),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(Vec::new())),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None), // this should not reached
    }
  }
}
