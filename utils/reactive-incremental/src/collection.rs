use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::{FastHashMap, FastHashSet};
use parking_lot::{RwLock, RwLockReadGuard};
use storage::IndexKeptVec;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  /// here we not impose any delta on
  Delta(K, V),
  Remove(K),
}

impl<K, V> CollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl FnOnce(&K, V) -> R) -> CollectionDelta<K, R> {
    type Rt<K, R> = CollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => {
        let mapped = mapper(&k, d);
        Rt::<K, R>::Delta(k, mapped)
      }
    }
  }

  pub fn value(self) -> Option<V> {
    match self {
      Self::Delta(_, v) => Some(v),
      Self::Remove(_) => None,
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

pub trait VirtualCollection<K, V> {
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_;

  fn iter_key_value(&self, skip_cache: bool) -> impl Iterator<Item = (K, V)> + '_ {
    let access = self.access(skip_cache);
    self.iter_key(skip_cache).map(move |k| {
      let v = access(&k).expect("iter_key_value provide key but not have valid value");
      (k, v)
    })
  }

  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  ///
  /// If the skip_cache is true, the implementation will not be incremental and will make sure the
  /// access is up to date.
  ///
  /// If the return is None, it means the value is not exist in the table.
  ///
  /// The implementation should guarantee it's ok to allow  multiple accessor instance exist in same
  /// time. (should only create read guard in underlayer)
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_;
}

pub trait VirtualMultiCollection<K, V> {
  fn iter_key_in_multi_collection(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_;
  fn access_multi(&self, skip_cache: bool) -> impl Fn(&K, &mut dyn FnMut(V)) + '_;
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine this is a data table with the K as the primary key and V as the row of the
/// data(not contains K). In this table, besides getting data, you can also poll it's partial
/// changes.
///
/// ## Implementation notes:
///
/// ### Compare to Stream
///
/// The first version of this trait is directly using the Stream as it's parent trait. But in
/// practice, this cause a lot trouble. We are using many unstable feature like impl trait in return
/// type, and impl trait in trait, our design require use to bound the stream's item with
/// IntoIterator, it's hard to express this trait bound everywhere because rust can not auto infer
/// it's bound requirement.
///
///
/// ### Extra design idea
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
///
/// ## Data Coherency
///
/// The data maybe slate if we combine these two trait directly because the visitor maybe not
/// directly access the original source data, but access the cache. This access abstract the
/// internal cache mechanism. Note, even if the polling issued before access, you still can not
/// guaranteed to access the "current" data due to the multi-threaded source mutation. Because of
/// this limitation, user should make sure their downstream consuming logic is timeline insensitive.
///
/// In the future, maybe we could add new sub-trait to enforce the data access is consistent with
/// the polling logic in tradeoff of the potential memory overhead.
pub trait ReactiveCollection<K, V>: VirtualCollection<K, V> + 'static {
  type Changes: IntoIterator<Item = CollectionDelta<K, V>>;
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>>;
}

// /// dynamic version of the above trait
// pub trait DynamicVirtualCollection<K, V> {
//   fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_>;
//   fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_>;
// }
// impl<K, V, T> DynamicVirtualCollection<K, V> for T
// where
//   Self: ReactiveCollection<K, V>,
// {
//   fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_> {
//     Box::new(self.iter_key(skip_cache))
//   }

//   fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_> {
//     Box::new(self.access(skip_cache))
//   }
// }
// pub trait DynamicReactiveCollection<K, V>: ReactiveCollection<K, V> {}

// impl<K, V> VirtualCollection<K, V> for &dyn DynamicReactiveCollection<K, V> {
//   fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
//     self.access_boxed(skip_cache)
//   }

//   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
//     self.iter_key_boxed(skip_cache)
//   }
// }

#[pin_project::pin_project]
struct ReactiveCollectionAsStream<T, K, V> {
  #[pin]
  inner: T,
  phantom: PhantomData<(K, V)>,
}

impl<K, V, T: ReactiveCollection<K, V> + Unpin> Stream for ReactiveCollectionAsStream<T, K, V> {
  type Item = T::Changes;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    this.inner.poll_changes(cx)
  }
}

pub trait ReactiveCollectionExt<K, V>: Sized + 'static + ReactiveCollection<K, V>
where
  V: 'static,
  K: 'static,
{
  fn into_change_stream(self) -> impl Stream<Item = Self::Changes>
  where
    Self: Unpin,
  {
    ReactiveCollectionAsStream {
      inner: self,
      phantom: PhantomData,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> V2 + Copy + 'static,
  {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> impl ReactiveCollection<K, V>
  where
    V: Copy,
    F: Fn(V) -> bool + 'static + Copy,
  {
    ReactiveKVFilter {
      inner: self,
      checker: move |v| if f(v) { Some(v) } else { None },
      k: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> Option<V2> + 'static + Copy,
  {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  fn collective_union<V2, Other>(
    self,
    other: Other,
  ) -> impl ReactiveCollection<K, (Option<V>, Option<V2>)>
  where
    Other: ReactiveCollection<K, V2>,
    K: Copy + std::hash::Hash + Eq,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      k: PhantomData,
    }
  }

  /// K should not overlap
  fn collective_select<Other>(self, other: Other) -> impl ReactiveCollection<K, V>
  where
    K: Copy + std::hash::Hash + Eq,
    Other: ReactiveCollection<K, V>,
  {
    self.collective_union(other).collective_map(selector)
  }

  /// K should fully overlap
  fn collective_zip<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq,
    Other: ReactiveCollection<K, V2>,
    V2: 'static,
  {
    self.collective_union(other).collective_map(zipper)
  }

  /// only return overlapped part
  fn collective_intersect<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveCollection<K, V2>,
    V2: 'static,
  {
    self
      .collective_union(other)
      .collective_filter_map(intersect_fn)
  }

  /// filter map<k, v> by reactive set<k>
  /// have to use box here to avoid complex type(could be improved)
  fn filter_by_keyset<S>(self, set: S) -> impl ReactiveCollection<K, V>
  where
    K: Copy + std::hash::Hash + Eq,
    S: ReactiveCollection<K, ()>,
  {
    self.collective_intersect(set).collective_map(|(v, _)| v)
  }

  fn into_table_forker(self) -> ReactiveKVMapFork<Self, K, V> {
    let (sender, rev) = single_value_channel();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    init.insert(id, sender);
    ReactiveKVMapFork {
      inner: Arc::new(RwLock::new(self)),
      downstream: Arc::new(RwLock::new(init)),
      rev,
      id,
      phantom: PhantomData,
    }
  }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveCollection<Self, K, V> {
    UnorderedMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }
  fn materialize_linear(self) -> LinearMaterializedReactiveCollection<Self, V> {
    LinearMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }
}
impl<T, K, V> ReactiveCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveCollection<K, V>,
  V: 'static,
  K: 'static,
{
}

fn selector<T>((a, b): (Option<T>, Option<T>)) -> T {
  match (a, b) {
    (Some(_), Some(_)) => unreachable!("key set should not overlap"),
    (Some(a), None) => a,
    (None, Some(b)) => b,
    (None, None) => unreachable!("value not selected"),
  }
}

fn zipper<T, U>((a, b): (Option<T>, Option<U>)) -> (T, U) {
  match (a, b) {
    (Some(a), Some(b)) => (a, b),
    _ => unreachable!("value not zipped"),
  }
}

fn intersect_fn<T, U>((a, b): (Option<T>, Option<U>)) -> Option<(T, U)> {
  match (a, b) {
    (Some(a), Some(b)) => Some((a, b)),
    _ => None,
  }
}

pub struct ReactiveKVMapFork<Map: ReactiveCollection<K, V>, K, V> {
  inner: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, reactive::SingleSender<Map::Changes>>>>,
  rev: reactive::SingleReceiver<Map::Changes>,
  id: u64,
  phantom: PhantomData<(K, V)>,
}

impl<Map: ReactiveCollection<K, V>, K, V> Drop for ReactiveKVMapFork<Map, K, V> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}
impl<Map: ReactiveCollection<K, V>, K, V> Clone for ReactiveKVMapFork<Map, K, V> {
  fn clone(&self) -> Self {
    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    // we don't expect clone in real runtime so we don't care about wake
    let (sender, rev) = single_value_channel();
    downstream.insert(id, sender);
    Self {
      inner: self.inner.clone(),
      downstream: self.downstream.clone(),
      id,
      phantom: PhantomData,
      rev,
    }
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  Map::Changes: Clone,
  K: 'static,
  V: 'static,
{
  type Changes = Map::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    // these writes should not deadlock, because we not prefer the concurrency between the table
    // updates. if we do allow it in the future, just change it to try write or yield pending.

    let r = self.rev.poll_next_unpin(cx);
    if r.is_ready() {
      return r;
    }

    let mut inner = self.inner.write();
    let r = inner.poll_changes(cx);

    if let Poll::Ready(Some(v)) = r {
      let downstream = self.downstream.write();
      for downstream in downstream.values() {
        downstream.update(v.clone()).ok();
      }
    }
    drop(inner);

    self.poll_changes(cx)
  }
}

impl<K, V, Map> VirtualCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    struct ReactiveKVMapForkRead<'a, Map, I> {
      _inner: RwLockReadGuard<'a, Map>,
      inner_iter: I,
    }

    impl<'a, Map, I: Iterator> Iterator for ReactiveKVMapForkRead<'a, Map, I> {
      type Item = I::Item;

      fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
      }
    }

    /// util to get collection's accessor type
    type IterOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Iterator<Item = K> + 'a;
    fn get_iter<'a, K, V, M>(map: &M, skip_cache: bool) -> IterOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.iter_key(skip_cache)
    }

    let inner = self.inner.read();
    let inner_iter = get_iter(inner.deref(), skip_cache);
    // safety: read guard is hold by iter, acc's real reference is form the Map
    let inner_iter: IterOf<'static, Map, K, V> = unsafe { std::mem::transmute(inner_iter) };
    ReactiveKVMapForkRead {
      _inner: inner,
      inner_iter,
    }
  }

  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.read();

    /// util to get collection's accessor type
    type AccessorOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Fn(&K) -> Option<V> + 'a;
    fn get_accessor<'a, K, V, M>(map: &M, skip_cache: bool) -> AccessorOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.access(skip_cache)
    }

    let acc: AccessorOf<Map, K, V> = get_accessor(inner.deref(), skip_cache);
    // safety: read guard is hold by closure, acc's real reference is form the Map
    let acc: AccessorOf<'static, Map, K, V> = unsafe { std::mem::transmute(acc) };
    move |key| {
      let _holder = &inner;
      let acc = &acc;
      acc(key)
    }
  }
}

pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  inner: Map,
  cache: FastHashMap<K, V>,
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  Map::Changes: Clone,
  K: std::hash::Hash + Eq + Clone + 'static,
  V: Clone + 'static,
{
  type Changes = Map::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.inner.poll_changes(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          CollectionDelta::Delta(k, v) => {
            self.cache.insert(k, v);
          }
          CollectionDelta::Remove(k) => {
            // todo, shrink
            self.cache.remove(&k);
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: VirtualCollection<K, V>,
  K: std::hash::Hash + Eq + Clone,
  V: Clone,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    if skip_cache {
      Box::new(self.inner.iter_key(skip_cache)) as Box<dyn Iterator<Item = K> + '_>
    } else {
      Box::new(self.cache.keys().cloned()) as Box<dyn Iterator<Item = K> + '_>
    }
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.get(key).cloned()
      }
    }
  }
}

pub struct LinearMaterializedReactiveCollection<Map, V> {
  inner: Map,
  cache: IndexKeptVec<V>,
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V>,
  Map::Changes: Clone,
  K: LinearIdentification + 'static,
  V: Clone + 'static,
{
  type Changes = Map::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.inner.poll_changes(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          CollectionDelta::Delta(k, v) => {
            self.cache.insert(v, k.alloc_index());
          }
          CollectionDelta::Remove(k) => {
            // todo, shrink
            self.cache.remove(k.alloc_index());
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: VirtualCollection<K, V>,
  K: LinearIdentification + 'static,
  V: Clone,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    if skip_cache {
      Box::new(self.inner.iter_key(skip_cache)) as Box<dyn Iterator<Item = K> + '_>
    } else {
      Box::new(self.cache.iter().map(|(k, _)| K::from_alloc_index(k)))
        as Box<dyn Iterator<Item = K> + '_>
    }
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.try_get(key.alloc_index()).cloned()
      }
    }
  }
}

pub struct ReactiveKVMap<T, F, K, V> {
  inner: T,
  map: F,
  phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  V: 'static,
  K: 'static,
  F: Fn(V) -> V2 + Copy + 'static,
  T: ReactiveCollection<K, V>,
{
  type Changes = impl IntoIterator<Item = CollectionDelta<K, V2>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let mapper = self.map;
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        deltas
          .into_iter()
          .map(move |delta| delta.map(|_, v| mapper(v)))
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key(skip_cache)
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).map(|v| (self.map)(v))
  }
}

pub struct ReactiveKVFilter<T, F, K, V> {
  inner: T,
  checker: F,
  k: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy + 'static,
  T: ReactiveCollection<K, V>,
  K: 'static,
  V: 'static,
{
  type Changes = impl IntoIterator<Item = CollectionDelta<K, V2>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let checker = self.checker;
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        deltas.into_iter().map(move |delta| match delta {
          CollectionDelta::Delta(k, v) => match checker(v) {
            Some(v) => CollectionDelta::Delta(k, v),
            None => CollectionDelta::Remove(k),
          },
          // the Remove variant maybe called many times for given k
          CollectionDelta::Remove(k) => CollectionDelta::Remove(k),
        })
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    self.inner.iter_key(skip_cache).filter(move |k| {
      let v = inner_getter(k).unwrap();
      (self.checker)(v).is_some()
    })
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).and_then(|v| (self.checker)(v))
  }
}

pub struct ReactiveKVUnion<T1, T2, K> {
  a: T1,
  b: T2,
  k: PhantomData<K>,
}

impl<T1, T2, K, V1, V2> VirtualCollection<K, (Option<V1>, Option<V2>)>
  for ReactiveKVUnion<T1, T2, K>
where
  K: Copy + std::hash::Hash + Eq,
  T1: VirtualCollection<K, V1>,
  T2: VirtualCollection<K, V2>,
{
  /// we require the T1 T2 has the same key range
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let mut keys = FastHashSet::<K>::default();
    self.a.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    self.b.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    keys.into_iter()
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<(Option<V1>, Option<V2>)> + '_ {
    let getter_a = self.a.access(skip_cache);
    let getter_b = self.b.access(skip_cache);

    move |key| Some((getter_a(key), getter_b(key)))
  }
}

impl<T1, T2, K, V1, V2> ReactiveCollection<K, (Option<V1>, Option<V2>)>
  for ReactiveKVUnion<T1, T2, K>
where
  K: Copy + std::hash::Hash + Eq + 'static,
  T1: ReactiveCollection<K, V1>,
  T2: ReactiveCollection<K, V2>,
{
  type Changes = impl IntoIterator<Item = CollectionDelta<K, (Option<V1>, Option<V2>)>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let t1 = self.a.poll_changes(cx);
    let t2 = self.b.poll_changes(cx);

    let a_access = self.a.access(false);
    let b_access = self.b.access(false);

    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let mut intersections: FastHashMap<K, (Option<V1>, Option<V2>)> = FastHashMap::default();
        v1.into_iter().for_each(|d| match d {
          CollectionDelta::Delta(k, v) => {
            intersections.entry(k).or_insert_with(Default::default).0 = Some(v);
          }
          CollectionDelta::Remove(k) => {
            intersections.entry(k).or_insert_with(Default::default).0 = None;
          }
        });
        v2.into_iter().for_each(|d| match d {
          CollectionDelta::Delta(k, v) => {
            intersections.entry(k).or_insert_with(Default::default).1 = Some(v);
          }
          CollectionDelta::Remove(k) => {
            intersections.entry(k).or_insert_with(Default::default).1 = None;
          }
        });

        let output = intersections
          .into_iter()
          .map(|(k, v)| {
            let v_map = match v {
              (Some(v1), Some(v2)) => (Some(v1), Some(v2)),
              (Some(v1), None) => (Some(v1), b_access(&k)),
              (None, Some(v2)) => (a_access(&k), Some(v2)),
              (None, None) => return CollectionDelta::Remove(k),
            };
            CollectionDelta::Delta(k, v_map)
          })
          .collect::<Vec<_>>();

        Poll::Ready(Some(output))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(
        v1.into_iter()
          .map(|v1| {
            let k = *v1.key();
            let v1 = v1.value();
            let v2 = b_access(&k);
            match (&v1, &v2) {
              (None, None) => CollectionDelta::Remove(k),
              _ => CollectionDelta::Delta(k, (v1, v2)),
            }
          })
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(
        v2.into_iter()
          .map(|v2| {
            let k = *v2.key();
            let v1 = a_access(&k);
            let v2 = v2.value();
            match (&v1, &v2) {
              (None, None) => CollectionDelta::Remove(k),
              _ => CollectionDelta::Delta(k, (v1, v2)),
            }
          })
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None),
    }
  }
}
