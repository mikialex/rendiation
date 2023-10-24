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
  pub fn map<R>(self, mapper: impl FnOnce(&K, V) -> R) -> VirtualKVCollectionDelta<K, R> {
    type Rt<K, R> = VirtualKVCollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => {
        let mapped = mapper(&k, d);
        Rt::<K, R>::Delta(k, mapped)
      }
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
  ///
  /// If the skip_cache is true, the implementation will not be incremental
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V> + '_;
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine this is a data table with the K as the primary key and V as the row of the
/// data(not contains K). In this table, besides getting data, you can also poll it's partial
/// changes.
///
/// Implementation notes:
///
/// This trait maybe could generalize to SignalLike trait:
/// ```rust
/// pub trait Signal<T: IncrementalBase>: Stream<Item = T::Delta> {
///   fn access(&self) -> T;
/// }
/// ```
/// However, this idea has not baked enough. For example, how do we express efficient partial
/// access for large T or container like T? Should we use some accessor associate trait or type as
/// the accessor key? Should we link this type to the T like how we did in Incremental trait?
pub trait ReactiveKVCollection<K, V>: VirtualKVCollection<K, V> + Stream + Unpin {}

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

/// dynamic version of the above trait
pub trait DynamicVirtualKVCollection<K, V> {
  fn access(&self, getter: &dyn FnOnce(&dyn Fn(K) -> Option<V>), skip_cache: bool);
}
pub trait DynamicReactiveKVCollection<K, V>:
  DynamicVirtualKVCollection<K, V> + Stream<Item = Vec<VirtualKVCollectionDelta<K, V>>> + Unpin
{
}
impl<K, V> VirtualKVCollection<K, V> for &dyn DynamicReactiveKVCollection<K, V> {
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V> + '_ {
    move |key| {
      let mut r = None;
      (*self).access(
        &|getter| {
          r = getter(key);
        },
        skip_cache,
      );
      r
    }
  }
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
      phantom: PhantomData,
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

  fn zip<V2, Other: ReactiveKVCollection<K, V2>>(
    self,
    other: Other,
  ) -> ReactiveKVZip<Self, Other, K> {
    ReactiveKVZip {
      a: self,
      b: other,
      k: PhantomData,
    }
  }

  fn select<Other: ReactiveKVCollection<K, V>>(
    self,
    other: Other,
  ) -> ReactiveKVSelect<Self, Other, K> {
    ReactiveKVSelect {
      a: self,
      b: other,
      k: PhantomData,
    }
  }

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
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.get(&key).cloned()
      }
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  phantom: PhantomData<(K, V)>,
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
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas
          .into_iter()
          .map(move |delta| delta.map(|_, v| mapper(v)))
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualKVCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).map(|v| (self.map)(v))
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
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).and_then(|v| (self.checker)(&v).then_some(v))
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

impl<T1, T2, K, V1, V2> VirtualKVCollection<K, (V1, V2)> for ReactiveKVZip<T1, T2, K>
where
  K: Copy,
  T1: VirtualKVCollection<K, V1>,
  T2: VirtualKVCollection<K, V2>,
{
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<(V1, V2)> + '_ {
    let getter_a = self.a.access(skip_cache);
    let getter_b = self.b.access(skip_cache);

    move |key| getter_a(key).zip(getter_b(key))
  }
}

impl<T1, T2, K, V1, V2> Stream for ReactiveKVZip<T1, T2, K>
where
  K: Clone,
  T1: Stream + Unpin,
  T1::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V1>>,
  T2: Stream + Unpin,
  T2::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>,
  // we require this to make the zip meaningful
  V1: IncrementalBase<Delta = V1>,
  V2: IncrementalBase<Delta = V2>,
  T1: VirtualKVCollection<K, V1>,
  T2: VirtualKVCollection<K, V2>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, (V1, V2)>>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let t1 = self.a.poll_next_unpin(cx);
    let t2 = self.b.poll_next_unpin(cx);

    let a_access = self.a.access(false);
    let b_access = self.b.access(false);

    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let intersections: FastHashMap<K, (Option<V1>, Option<V2>)> = FastHashMap::default();
        v1.into_iter().for_each(|d| {
          match d {
            VirtualKVCollectionDelta::Delta(K, V) => {
              //
            }
            VirtualKVCollectionDelta::Remove(K) => {
              //
            }
          }
        });

        Poll::Ready(Some(Vec::new()))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(
        v1.into_iter()
          .map(|v1| v1.map(|k, v| (v, b_access(k.clone()).unwrap())))
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(
        v2.into_iter()
          .map(|v2| v2.map(|k, v| (a_access(k.clone()).unwrap(), v)))
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None), // this should not reached
    }
  }
}

/// T1, T2's K should not overlap
#[pin_project::pin_project]
pub struct ReactiveKVSelect<T1, T2, K> {
  #[pin]
  a: T1,
  #[pin]
  b: T2,
  k: PhantomData<K>,
}

impl<T1, T2, K, V> VirtualKVCollection<K, V> for ReactiveKVSelect<T1, T2, K>
where
  K: Copy,
  T1: VirtualKVCollection<K, V>,
  T2: VirtualKVCollection<K, V>,
{
  fn access(&self, skip_cache: bool) -> impl Fn(K) -> Option<V> + '_ {
    let getter_a = self.a.access(skip_cache);
    let getter_b = self.b.access(skip_cache);

    move |key| getter_a(key).or(getter_b(key))
  }
}

impl<T1, T2, K, V> Stream for ReactiveKVSelect<T1, T2, K>
where
  T1: Stream,
  T1::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
  T2: Stream,
  T2::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();

    let t1 = this.a.poll_next(cx);
    let t2 = this.b.poll_next(cx);

    // todo, avoid collect!
    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        Poll::Ready(Some(v1.into_iter().chain(v2).collect::<Vec<_>>()))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(v1.into_iter().collect())),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(v2.into_iter().collect())),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None), // this should not reached
    }
  }
}
