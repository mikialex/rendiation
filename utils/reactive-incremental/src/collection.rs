use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::{FastHashMap, FastHashSet};
use parking_lot::{RwLock, RwLockReadGuard};
use storage::IndexKeptVec;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  // k, new_v, pre_v
  Delta(K, V, Option<V>),
  // k, pre_v
  Remove(K, V),
}

impl<K, V> CollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl Fn(&K, V) -> R) -> CollectionDelta<K, R> {
    type Rt<K, R> = CollectionDelta<K, R>;
    match self {
      Self::Remove(k, pre) => {
        let mapped = mapper(&k, pre);
        Rt::<K, R>::Remove(k, mapped)
      }
      Self::Delta(k, d, pre) => {
        let mapped = mapper(&k, d);
        let mapped_pre = pre.map(|d| mapper(&k, d));
        Rt::<K, R>::Delta(k, mapped, mapped_pre)
      }
    }
  }

  pub fn new_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, v, _) => Some(v),
      Self::Remove(_, _) => None,
    }
  }

  pub fn old_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, _, Some(v)) => Some(v),
      Self::Remove(_, v) => Some(v),
      _ => None,
    }
  }

  // should we just use struct??
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k, _) => k,
      Self::Delta(k, _, _) => k,
    }
  }
}

pub trait VirtualCollection<K, V> {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    let access = self.access();
    self.iter_key().map(move |k| {
      let v = access(&k).expect("iter_key_value provide key but not have valid value");
      (k, v)
    })
  }

  /// Access the current value. we use this accessor like api style for fast batch accessing(to
  /// avoid internal fragmented locking). the returned V is passed by ownership because we may
  /// create data on the fly.
  ///
  /// If the return value is None, it means the value does not exist in the table.
  ///
  /// The implementation should guarantee that it's ok to allow multiple accessor instances coexists
  /// at the same time. (should only create read guard in underlayer)
  fn access(&self) -> impl Fn(&K) -> Option<V> + '_;
}

pub trait VirtualMultiCollection<K, V> {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_;
  fn access_multi(&self) -> impl Fn(&K, &mut dyn FnMut(V)) + '_;
}

pub trait DynamicVirtualMultiCollection<O, M> {
  fn iter_key_in_multi_collection_boxed(&self) -> Box<dyn Iterator<Item = O> + '_>;
  fn access_multi_boxed(&self) -> Box<dyn Fn(&O, &mut dyn FnMut(M)) + '_>;
}
impl<T, O, M> DynamicVirtualMultiCollection<O, M> for T
where
  T: VirtualMultiCollection<O, M>,
{
  fn iter_key_in_multi_collection_boxed(&self) -> Box<dyn Iterator<Item = O> + '_> {
    Box::new(self.iter_key_in_multi_collection())
  }
  fn access_multi_boxed(&self) -> Box<dyn Fn(&O, &mut dyn FnMut(M)) + '_> {
    Box::new(self.access_multi())
  }
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine that this is a data table with the K as the primary key and V as the row of the
/// data(not containing K). In this table, besides getting data, you can also poll it's partial
/// changes.
///
/// ## Implementation notes:
///
/// ### Compare to Stream
///
/// The first version of this trait is directly using the Stream as it's parent trait. But in
/// practice, this cause a lot trouble. We are using many unstable feature like impl trait in return
/// type, and impl trait in trait, our design require use to bound the stream's item with
/// IntoIterator, it's hard to express this trait bound everywhere because the current rust can not
/// auto infer it's bound requirement.
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
/// The implementation should guarantee that the data access in VirtualCollection trait should be
/// coherent with the change polling. For example, if the change has not been polled, the accessor
/// should access the slate data but not the current.
pub trait ReactiveCollection<K, V>: VirtualCollection<K, V> + 'static {
  type Changes: Iterator<Item = CollectionDelta<K, V>> + Clone;
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);
}

pub enum ExtraCollectionOperation {
  MemoryShrinkToFit,
}

/// it's useful to use () as the empty collection
impl<K: 'static, V> VirtualCollection<K, V> for () {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    [].into_iter()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    |_| None
  }
}

pub struct EmptyIter<T>(PhantomData<T>);

impl<T> Clone for EmptyIter<T> {
  fn clone(&self) -> Self {
    Self(PhantomData)
  }
}
impl<T> Iterator for EmptyIter<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    None
  }
}
impl<K: 'static, V> ReactiveCollection<K, V> for () {
  type Changes = EmptyIter<CollectionDelta<K, V>>;

  fn poll_changes(&mut self, _: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    Poll::Pending
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}

/// dynamic version of the above trait
pub trait DynamicVirtualCollection<K, V> {
  fn iter_key_boxed(&self) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_boxed(&self) -> Box<dyn Fn(&K) -> Option<V> + '_>;
}
impl<K, V, T> DynamicVirtualCollection<K, V> for T
where
  Self: VirtualCollection<K, V>,
{
  fn iter_key_boxed(&self) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new(self.iter_key())
  }

  fn access_boxed(&self) -> Box<dyn Fn(&K) -> Option<V> + '_> {
    Box::new(self.access())
  }
}

pub type BoxedReactiveCollectionDeltaTy<K, V> = Box<dyn DynamicReactiveCollectionDelta<K, V>>;
pub trait DynamicReactiveCollectionDelta<K, V>: Iterator<Item = CollectionDelta<K, V>> {
  fn clone_dyn(&self) -> Box<dyn DynamicReactiveCollectionDelta<K, V>>;
}
impl<K, V, T> DynamicReactiveCollectionDelta<K, V> for T
where
  T: Iterator<Item = CollectionDelta<K, V>> + Clone + 'static,
{
  fn clone_dyn(&self) -> Box<dyn DynamicReactiveCollectionDelta<K, V>> {
    Box::new(self.clone())
  }
}
impl<K, V> Clone for BoxedReactiveCollectionDeltaTy<K, V>
where
  K: 'static,
  V: 'static,
{
  fn clone(&self) -> Self {
    self.clone_dyn()
  }
}

pub trait DynamicReactiveCollection<K, V>: DynamicVirtualCollection<K, V> {
  fn poll_changes_dyn(
    &mut self,
    _cx: &mut Context<'_>,
  ) -> Poll<Option<BoxedReactiveCollectionDeltaTy<K, V>>>;
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K, V, T> DynamicReactiveCollection<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: 'static,
  V: 'static,
{
  fn poll_changes_dyn(
    &mut self,
    cx: &mut Context<'_>,
  ) -> Poll<Option<BoxedReactiveCollectionDeltaTy<K, V>>> {
    self
      .poll_changes(cx)
      .map(|v| v.map(|v| Box::new(v) as BoxedReactiveCollectionDeltaTy<K, V>))
  }
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<K, V> VirtualCollection<K, V> for Box<dyn DynamicReactiveCollection<K, V>> {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.iter_key_boxed()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    self.access_boxed()
  }
}
impl<K, V> ReactiveCollection<K, V> for Box<dyn DynamicReactiveCollection<K, V>>
where
  K: Clone + 'static,
  V: Clone + 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<K, V>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request_dyn(request)
  }
}

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
  V: Clone + 'static,
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

  /// map map<k, v> to map<k, v2>
  fn collective_execute_map_by<V2, F, FF>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn() -> FF + 'static,
    FF: Fn(&K, V) -> V2 + 'static,
    K: Clone,
    V2: Clone,
  {
    ReactiveKVExecuteMap {
      inner: self,
      map_creator: f,
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
    V2: Clone,
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
    self
      .collective_union(other)
      .collective_map(|(a, b)| match (a, b) {
        (Some(_), Some(_)) => unreachable!("key set should not overlap"),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => unreachable!("value not selected"),
      })
  }

  /// K should fully overlap
  fn collective_zip<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq,
    Other: ReactiveCollection<K, V2>,
    V2: Clone + 'static,
  {
    self
      .collective_union(other)
      .collective_map(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => (a, b),
        _ => unreachable!("value not zipped"),
      })
  }

  /// only return overlapped part
  fn collective_intersect<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveCollection<K, V2>,
    V2: Clone + 'static,
  {
    self
      .collective_union(other)
      .collective_filter_map(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
      })
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

  fn into_forker(self) -> ReactiveKVMapFork<Self, K, V> {
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
  V: Clone + 'static,
  K: 'static,
{
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

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.write().extra_request(request)
  }
}

impl<K, V, Map> VirtualCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
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
    fn get_iter<'a, K, V, M>(map: &M) -> IterOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.iter_key()
    }

    let inner = self.inner.read();
    let inner_iter = get_iter(inner.deref());
    // safety: read guard is hold by iter, acc's real reference is form the Map
    let inner_iter: IterOf<'static, Map, K, V> = unsafe { std::mem::transmute(inner_iter) };
    ReactiveKVMapForkRead {
      _inner: inner,
      inner_iter,
    }
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.read();

    /// util to get collection's accessor type
    type AccessorOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Fn(&K) -> Option<V> + 'a;
    fn get_accessor<'a, K, V, M>(map: &M) -> AccessorOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.access()
    }

    let acc: AccessorOf<Map, K, V> = get_accessor(inner.deref());
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
      for change in changes.clone() {
        match change {
          CollectionDelta::Delta(k, v, _) => {
            self.cache.insert(k, v);
          }
          CollectionDelta::Remove(k, _) => {
            self.cache.remove(&k);
          }
        }
      }
    }
    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.shrink_to_fit(),
    }
  }
}

impl<K, V, Map> VirtualCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: VirtualCollection<K, V>,
  K: std::hash::Hash + Eq + Clone,
  V: Clone,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.cache.keys().cloned()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    move |key| self.cache.get(key).cloned()
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
      for change in changes.clone() {
        match change {
          CollectionDelta::Delta(k, v, _) => {
            self.cache.insert(v, k.alloc_index());
          }
          CollectionDelta::Remove(k, _) => {
            self.cache.remove(k.alloc_index());
          }
        }
      }
    }
    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.shrink_to_fit(),
    }
  }
}

impl<K, V, Map> VirtualCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: VirtualCollection<K, V>,
  K: LinearIdentification + 'static,
  V: Clone,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.cache.iter().map(|(k, _)| K::from_alloc_index(k))
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    move |key| self.cache.try_get(key.alloc_index()).cloned()
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T, F, K, V> {
  inner: T,
  map_creator: F,
  phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2, FF> ReactiveCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V>
where
  V: 'static,
  K: Clone + 'static,
  F: Fn() -> FF + 'static,
  FF: Fn(&K, V) -> V2 + 'static,
  V2: Clone,
  T: ReactiveCollection<K, V>,
{
  type Changes = impl Iterator<Item = CollectionDelta<K, V2>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        let mapper = (self.map_creator)();
        deltas
          .into_iter()
          .map(move |delta| delta.map(|k, v| mapper(k, v)))
          .collect::<Vec<_>>()
          .into_iter()
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, F, FF, K, V, V2> VirtualCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V>
where
  F: Fn() -> FF + 'static,
  FF: Fn(&K, V) -> V2 + 'static,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access();
    let mapper = (self.map_creator)();
    move |key| inner_getter(key).map(|v| mapper(key, v))
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
  type Changes = impl Iterator<Item = CollectionDelta<K, V2>> + Clone;

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

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access();
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
  type Changes = impl Iterator<Item = CollectionDelta<K, V2>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let checker = self.checker;
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        deltas.into_iter().filter_map(move |delta| {
          match delta {
            CollectionDelta::Delta(k, v, pre_v) => {
              let new_map = checker(v);
              let pre_map = pre_v.and_then(checker);
              match (new_map, pre_map) {
                (Some(v), Some(pre_v)) => CollectionDelta::Delta(k, v, Some(pre_v)),
                (Some(v), None) => CollectionDelta::Delta(k, v, None),
                (None, Some(pre_v)) => CollectionDelta::Remove(k, pre_v),
                (None, None) => return None,
              }
              .into()
            }
            // the Remove variant maybe called many times for given k
            CollectionDelta::Remove(k, pre_v) => {
              let pre_map = checker(pre_v);
              match pre_map {
                Some(pre) => CollectionDelta::Remove(k, pre).into(),
                None => None,
              }
            }
          }
        })
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy,
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    let inner_getter = self.inner.access();
    self.inner.iter_key().filter(move |k| {
      let v = inner_getter(k).unwrap();
      (self.checker)(v).is_some()
    })
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access();
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
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    let mut keys = FastHashSet::<K>::default();
    self.a.iter_key().for_each(|k| {
      keys.insert(k);
    });
    self.b.iter_key().for_each(|k| {
      keys.insert(k);
    });
    keys.into_iter()
  }
  fn access(&self) -> impl Fn(&K) -> Option<(Option<V1>, Option<V2>)> + '_ {
    let getter_a = self.a.access();
    let getter_b = self.b.access();

    move |key| Some((getter_a(key), getter_b(key)))
  }
}

/// we should manually impl zip, intersect, select, to avoid overhead
fn union<K: Clone, V1: Clone, V2: Clone>(
  change1: Option<CollectionDelta<K, V1>>,
  change2: Option<CollectionDelta<K, V2>>,
  v1_current: &impl Fn(&K) -> Option<V1>,
  v2_current: &impl Fn(&K) -> Option<V2>,
) -> Option<CollectionDelta<K, (Option<V1>, Option<V2>)>> {
  let r = match (change1, change2) {
    (None, None) => return None,
    (None, Some(change2)) => match change2 {
      CollectionDelta::Delta(k, v2, p2) => {
        let v1_current = v1_current(&k);
        CollectionDelta::Delta(k, (v1_current.clone(), Some(v2)), Some((v1_current, p2)))
      }
      CollectionDelta::Remove(k, p2) => {
        if let Some(v1_current) = v1_current(&k) {
          CollectionDelta::Delta(
            k,
            (Some(v1_current.clone()), None),
            Some((Some(v1_current), Some(p2))),
          )
        } else {
          CollectionDelta::Remove(k, (None, Some(p2)))
        }
      }
    },
    (Some(change1), None) => match change1 {
      CollectionDelta::Delta(k, v1, p1) => {
        let v2_current = v2_current(&k);
        CollectionDelta::Delta(k, (Some(v1), v2_current.clone()), Some((p1, v2_current)))
      }
      CollectionDelta::Remove(k, p1) => {
        if let Some(v2_current) = v2_current(&k) {
          CollectionDelta::Delta(
            k,
            (None, Some(v2_current.clone())),
            Some((Some(p1), Some(v2_current))),
          )
        } else {
          CollectionDelta::Remove(k, (Some(p1), None))
        }
      }
    },
    (Some(change1), Some(change2)) => match (change1, change2) {
      (CollectionDelta::Delta(k, v1, p1), CollectionDelta::Delta(_, v2, p2)) => {
        CollectionDelta::Delta(k, (Some(v1), Some(v2)), Some((p1, p2)))
      }
      (CollectionDelta::Delta(_, v1, p1), CollectionDelta::Remove(k, p2)) => {
        CollectionDelta::Delta(k.clone(), (Some(v1), v2_current(&k)), Some((p1, Some(p2))))
      }
      (CollectionDelta::Remove(k, p1), CollectionDelta::Delta(_, v2, p2)) => {
        CollectionDelta::Delta(k.clone(), (v1_current(&k), Some(v2)), Some((Some(p1), p2)))
      }
      (CollectionDelta::Remove(k, p1), CollectionDelta::Remove(_, p2)) => {
        CollectionDelta::Remove(k, (Some(p1), Some(p2)))
      }
    },
  };

  if let CollectionDelta::Delta(k, new, Some((None, None))) = r {
    return CollectionDelta::Delta(k, new, None).into();
  }

  r.into()
}

impl<T1, T2, K, V1, V2> ReactiveCollection<K, (Option<V1>, Option<V2>)>
  for ReactiveKVUnion<T1, T2, K>
where
  K: Copy + std::hash::Hash + Eq + 'static,
  T1: ReactiveCollection<K, V1>,
  T2: ReactiveCollection<K, V2>,
  V1: Clone,
  V2: Clone,
{
  type Changes = impl Iterator<Item = CollectionDelta<K, (Option<V1>, Option<V2>)>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let t1 = self.a.poll_changes(cx);
    let t2 = self.b.poll_changes(cx);

    let a_access = self.a.access();
    let b_access = self.b.access();

    let r = match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let mut intersections: FastHashMap<
          K,
          (
            Option<CollectionDelta<K, V1>>,
            Option<CollectionDelta<K, V2>>,
          ),
        > = FastHashMap::default();
        v1.for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().0 = Some(d)
        });

        v2.for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().1 = Some(d)
        });

        intersections
          .into_iter()
          .filter_map(|(_, (d1, d2))| union(d1, d2, &a_access, &b_access))
          .collect::<Vec<_>>()
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => v1
        .filter_map(|d1| union(Some(d1), None, &a_access, &b_access))
        .collect::<Vec<_>>(),
      (Poll::Pending, Poll::Ready(Some(v2))) => v2
        .filter_map(|d2| union(None, Some(d2), &a_access, &b_access))
        .collect::<Vec<_>>(),

      (Poll::Pending, Poll::Pending) => return Poll::Pending,
      _ => return Poll::Ready(None),
    };

    if r.is_empty() {
      return Poll::Pending;
    }

    Poll::Ready(Some(r.into_iter()))
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.a.extra_request(request);
    self.b.extra_request(request);
  }
}

/// when we want to zip multiple kv, using deeply nested zipper is viable, however it's computation
/// intensive during layer of layers consuming. This combinator provides the flattened version of
/// multi zip, in trades of the performance overhead of dynamical fn call and internal cache(maybe
/// user still require this so it's ok).
pub struct MultiZipper<K, P, V> {
  sources: Vec<Box<dyn DynamicReactiveCollection<K, P>>>,
  current: FastHashMap<K, V>,
  defaulter: Box<dyn Fn() -> V>,
  applier: Box<dyn Fn(&mut V, P)>,
}

impl<K, P, V> MultiZipper<K, P, V> {
  pub fn new(defaulter: impl Fn() -> V + 'static, applier: impl Fn(&mut V, P) + 'static) -> Self {
    Self {
      sources: Default::default(),
      current: Default::default(),
      defaulter: Box::new(defaulter),
      applier: Box::new(applier),
    }
  }

  pub fn zip_with(mut self, source: impl ReactiveCollection<K, P>) -> Self
  where
    K: 'static,
    P: 'static,
  {
    self.sources.push(Box::new(source));
    self
  }
}

impl<K, P, V> VirtualCollection<K, V> for MultiZipper<K, P, V>
where
  K: Clone + Eq + std::hash::Hash,
  V: Clone,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.current.keys().cloned()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    |k| self.current.get(k).cloned()
  }
}

impl<K, P, V> ReactiveCollection<K, V> for MultiZipper<K, P, V>
where
  K: Clone + Eq + std::hash::Hash + 'static,
  V: Clone + 'static,
  P: 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<K, V>> + Clone;

  #[allow(clippy::collapsible_else_if)]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    if self.sources.is_empty() {
      return Poll::Pending;
    }

    let mut outputs = FastHashMap::<K, CollectionDelta<K, V>>::default();

    for source in &mut self.sources {
      if let Poll::Ready(Some(source)) = source.poll_changes_dyn(cx) {
        for change in source {
          match change {
            CollectionDelta::Delta(key, change, _) => {
              if let Some(previous_new) = outputs.remove(&key) {
                match previous_new {
                  CollectionDelta::Delta(_, mut next, pre) => {
                    (self.applier)(&mut next, change);
                    outputs.insert(key.clone(), CollectionDelta::Delta(key.clone(), next, pre));
                  }
                  CollectionDelta::Remove(_, _) => unreachable!("unexpected zipper input"),
                }
                outputs.insert(
                  key.clone(),
                  CollectionDelta::Delta(key, (self.defaulter)(), None),
                );
              } else {
                if let Some(current) = self.current.get(&key) {
                  let mut next = current.clone();
                  (self.applier)(&mut next, change);
                  outputs.insert(
                    key.clone(),
                    CollectionDelta::Delta(key, next, Some(current.clone())),
                  );
                } else {
                  let mut next = (self.defaulter)();
                  (self.applier)(&mut next, change);
                  outputs.insert(key.clone(), CollectionDelta::Delta(key, next, None));
                }
              }
            }
            CollectionDelta::Remove(key, _) => {
              outputs.insert(
                key.clone(),
                CollectionDelta::Remove(key.clone(), self.current.remove(&key).unwrap()),
              );
            }
          }
        }
      }
    }

    if outputs.is_empty() {
      return Poll::Pending;
    }

    for v in outputs.values() {
      match v {
        CollectionDelta::Delta(k, next, _) => {
          self.current.insert(k.clone(), next.clone());
        }
        CollectionDelta::Remove(k, _) => {
          self.current.remove(k);
        }
      }
    }

    Poll::Ready(Some(HashMapIntoIter::new(outputs)))
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self
      .sources
      .iter_mut()
      .for_each(|s| s.extra_request_dyn(request));
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.current.shrink_to_fit(),
    }
  }
}

/// once hashmap into_values starts iterate, it's not able to clone again, so here i impl this to
/// workaround: as long as the into_values not call, the iter could be cloned.

pub(crate) enum HashMapIntoIter<K, V> {
  NotIter(FastHashMap<K, V>),
  Iter(std::collections::hash_map::IntoValues<K, V>),
}

impl<K, V> HashMapIntoIter<K, V> {
  pub fn new(map: FastHashMap<K, V>) -> Self {
    Self::NotIter(map)
  }
}

impl<K: Clone, V: Clone> Clone for HashMapIntoIter<K, V> {
  fn clone(&self) -> Self {
    match self {
      Self::NotIter(arg0) => Self::NotIter(arg0.clone()),
      Self::Iter(_) => panic!("hashmap iter should be cloned before do any iter"),
    }
  }
}

impl<K, V> Iterator for HashMapIntoIter<K, V> {
  type Item = V;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      HashMapIntoIter::NotIter(map) => {
        let map = std::mem::take(map);
        *self = HashMapIntoIter::Iter(map.into_values());
        self.next()
      }
      HashMapIntoIter::Iter(iter) => iter.next(),
    }
  }
}
