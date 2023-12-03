use std::{marker::PhantomData, ops::DerefMut, sync::Arc};

use dashmap::*;
use fast_hash_collection::*;
use storage::IndexKeptVec;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  // k, new_v
  Delta(K, V),
  // k
  Remove(K),
}

impl<K, V> CollectionDelta<K, V> {
  pub fn merge(self, later: Self) -> Option<Self>
  where
    K: Eq,
  {
    use CollectionDelta::*;
    if self.key() != later.key() {
      panic!("only same key change could be merge");
    }
    match (self, later) {
      // later override earlier
      (Delta(k, _d1), Delta(_, d2)) => Delta(k, d2),
      // later override earlier
      // if init not exist, remove is still allowed to be multiple
      (Delta(k, _d1), Remove(_)) => Remove(k),
      // later override earlier
      (Remove(k), Delta(_, d1)) => Delta(k, d1),
      // remove is allowed to be multiple
      (Remove(k), Remove(_)) => Remove(k),
    }
    .into()
  }
  pub fn map<R>(self, mapper: impl Fn(&K, V) -> R) -> CollectionDelta<K, R> {
    type Rt<K, R> = CollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => {
        let mapped = mapper(&k, d);
        Rt::<K, R>::Delta(k, mapped)
      }
    }
  }

  pub fn new_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, v) => Some(v),
      Self::Remove(_) => None,
    }
  }

  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k) => k,
      Self::Delta(k, _) => k,
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
  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_;
}

pub trait VirtualMultiCollection<K, V> {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_;
  fn access_multi(&self) -> impl Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_;
}

pub trait DynamicVirtualMultiCollection<O, M> {
  fn iter_key_in_multi_collection_boxed(&self) -> Box<dyn Iterator<Item = O> + '_>;
  fn access_multi_boxed(&self) -> Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>;
}
impl<T, O, M> DynamicVirtualMultiCollection<O, M> for T
where
  T: VirtualMultiCollection<O, M>,
{
  fn iter_key_in_multi_collection_boxed(&self) -> Box<dyn Iterator<Item = O> + '_> {
    Box::new(self.iter_key_in_multi_collection())
  }
  fn access_multi_boxed(&self) -> Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_> {
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
pub trait ReactiveCollection<K: Send, V: Send>:
  VirtualCollection<K, V> + Sync + Send + 'static
{
  type Changes: CollectionChanges<K, V>;
  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<Self::Changes>>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn poll_changes_and_merge_until_pending(
    &mut self,
    cx: &mut Context,
  ) -> Poll<Option<FastPassingVec<CollectionDelta<K, V>>>>
  where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
  {
    // we special check the first case to avoid merge cost if only has one yield
    let first = self
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()));

    if let Poll::Ready(Some(r)) = self.poll_changes(cx) {
      let r = r.collect_into_pass_vec();
      let mut hash = FastHashMap::default();

      if let Poll::Ready(Some(v)) = first {
        deduplicate_collection_changes(&mut hash, v.vec.as_slice().iter().cloned());
      }
      deduplicate_collection_changes(&mut hash, r.vec.as_slice().iter().cloned());

      while let Poll::Ready(Some(v)) = self.poll_changes(cx) {
        let v = v.collect_into_pass_vec();
        deduplicate_collection_changes(&mut hash, v.vec.as_slice().iter().cloned());
      }

      let vec: Vec<_> = hash.into_values().collect();
      if vec.is_empty() {
        Poll::Pending
      } else {
        Poll::Ready(Some(FastPassingVec::from_vec(vec)))
      }
    } else {
      first
    }
  }
}

pub fn deduplicate_collection_changes<K, V>(
  deduplicate: &mut FastHashMap<K, CollectionDelta<K, V>>,
  deltas: impl Iterator<Item = CollectionDelta<K, V>>,
) where
  K: Eq + std::hash::Hash + Clone,
  V: Clone,
{
  deltas.for_each(|d| {
    let key = d.key().clone();
    if let Some(current) = deduplicate.get_mut(&key) {
      if let Some(merged) = current.clone().merge(d) {
        *current = merged;
      } else {
        deduplicate.remove(&key);
      }
    } else {
      deduplicate.insert(key, d);
    }
  })
}

/// impl note: why not using IntoParallelIterator?
///
/// 1. rayon IntoParallelIterator's Iter associate type do not have sync bound
/// 2. we use return-type-impl-trait extensively, and could not express the parent trait's associate
/// type extra trait bound. maybe we could workaround this, but the code is awful. (that also the
/// reason why we impose the clone bound here)
pub trait CollectionChanges<K: Send, V: Send>:
  ParallelIterator<Item = CollectionDelta<K, V>>
  + MaybeFastCollect<CollectionDelta<K, V>>
  + Clone
  + Send
  + Sync
  + Sized
{
}
pub trait MaybeFastCollect<T: Send>: ParallelIterator<Item = T> + Sized {
  fn collect_into_pass_vec(self) -> FastPassingVec<T>;
}

impl<X: Send + Sync + Clone, T: ParallelIterator<Item = X>> MaybeFastCollect<X> for T {
  default fn collect_into_pass_vec(self) -> FastPassingVec<X> {
    FastPassingVec::from_vec(self.collect::<Vec<_>>())
  }
}

impl<K, V, T> CollectionChanges<K, V> for T
where
  K: Send,
  V: Send,
  T: ParallelIterator<Item = CollectionDelta<K, V>> + Send + Sync,
  T: MaybeFastCollect<CollectionDelta<K, V>>,
  T: Clone,
{
}

#[derive(Clone, Debug)]
pub struct FastPassingVec<T> {
  pub vec: Arc<Vec<T>>,
}

impl<T> FastPassingVec<T> {
  pub fn from_vec(vec: Vec<T>) -> Self {
    Self { vec: Arc::new(vec) }
  }
}

impl<T: Clone> IntoIterator for FastPassingVec<T> {
  type Item = T;
  type IntoIter = impl Iterator<Item = T>;
  fn into_iter(self) -> Self::IntoIter {
    // avoid heap to heap clone
    #[derive(Clone)]
    struct CheapCloneVecIter<T> {
      inner: Arc<Vec<T>>,
      next: usize,
    }
    impl<T: Clone> Iterator for CheapCloneVecIter<T> {
      type Item = T;
      fn next(&mut self) -> Option<Self::Item> {
        let v = self.inner.get(self.next).cloned();
        self.next += 1;
        v
      }
    }

    CheapCloneVecIter {
      inner: self.vec,
      next: 0,
    }
  }
}

impl<T: Send + Sync + Clone> ParallelIterator for FastPassingVec<T> {
  type Item = T;

  fn drive_unindexed<C>(self, consumer: C) -> C::Result
  where
    C: ::rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
  {
    // todo, how to avoid heap to heap clone?
    let vec = Vec::from(self.vec.as_slice());
    vec.into_par_iter().drive_unindexed(consumer)
  }
}
impl<T: Send + Sync + Clone> MaybeFastCollect<T> for FastPassingVec<T> {
  // when message type is already fast vec, collect is free of cost.
  fn collect_into_pass_vec(self) -> FastPassingVec<T> {
    self
  }
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
impl<T: Send + Sync + Clone> MaybeFastCollect<T> for EmptyIter<T> {
  fn collect_into_pass_vec(self) -> FastPassingVec<T> {
    FastPassingVec::from_vec(Default::default())
  }
}
unsafe impl<T> Send for EmptyIter<T> {}
unsafe impl<T> Sync for EmptyIter<T> {}

impl<T> Clone for EmptyIter<T> {
  fn clone(&self) -> Self {
    Self(PhantomData)
  }
}
impl<T: Send> ParallelIterator for EmptyIter<T> {
  type Item = T;

  fn drive_unindexed<C>(self, consumer: C) -> C::Result
  where
    C: ::rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
  {
    [].into_par_iter().drive_unindexed(consumer)
  }
}
impl<K, V> ReactiveCollection<K, V> for ()
where
  K: 'static + Send + Sync + Clone,
  V: 'static + Send + Sync + Clone,
{
  type Changes = EmptyIter<CollectionDelta<K, V>>;

  fn poll_changes(&mut self, _: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    Poll::Pending
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}

pub struct ConstCollection<T>(pub T);
impl<K: 'static, V> VirtualCollection<K, V> for ConstCollection<V> {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    struct FakeIter<T>(PhantomData<T>);
    impl<T> Iterator for FakeIter<T> {
      type Item = T;

      fn next(&mut self) -> Option<Self::Item> {
        None
      }
    }
    panic!("not able to iter all possible K values");
    #[allow(unreachable_code)]
    FakeIter::<K>(Default::default())
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    |_| None
  }
}
impl<K, V> ReactiveCollection<K, V> for ConstCollection<V>
where
  K: 'static + Send + Sync + Clone,
  V: 'static + Send + Sync + Clone,
{
  type Changes = EmptyIter<CollectionDelta<K, V>>;

  fn poll_changes(&mut self, _: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    Poll::Pending
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}

/// dynamic version of the above trait
pub trait DynamicVirtualCollection<K, V> {
  fn iter_key_boxed(&self) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_boxed(&self) -> Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
}
impl<K, V, T> DynamicVirtualCollection<K, V> for T
where
  Self: VirtualCollection<K, V>,
{
  fn iter_key_boxed(&self) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new(self.iter_key())
  }

  fn access_boxed(&self) -> Box<dyn Fn(&K) -> Option<V> + Sync + '_> {
    Box::new(self.access())
  }
}

pub trait DynamicReactiveCollection<K, V>: DynamicVirtualCollection<K, V> + Sync + Send {
  fn poll_changes_dyn(
    &mut self,
    _cx: &mut Context<'_>,
  ) -> Poll<Option<FastPassingVec<CollectionDelta<K, V>>>>;
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K, V, T> DynamicReactiveCollection<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: Send + 'static,
  V: Send + 'static,
{
  fn poll_changes_dyn(
    &mut self,
    cx: &mut Context<'_>,
  ) -> Poll<Option<FastPassingVec<CollectionDelta<K, V>>>> {
    self
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()))
  }
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<K, V> VirtualCollection<K, V> for Box<dyn DynamicReactiveCollection<K, V>> {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.deref().iter_key_boxed()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    self.deref().access_boxed()
  }
}
impl<K, V> ReactiveCollection<K, V> for Box<dyn DynamicReactiveCollection<K, V>>
where
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChanges<K, V>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.deref_mut().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}

#[pin_project::pin_project]
struct ReactiveCollectionAsStream<T, K, V> {
  #[pin]
  inner: T,
  phantom: PhantomData<(K, V)>,
}

impl<K, V, T> Stream for ReactiveCollectionAsStream<T, K, V>
where
  T: ReactiveCollection<K, V> + Unpin,
  K: Send,
  V: Send,
{
  type Item = T::Changes;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    this.inner.poll_changes(cx)
  }
}

pub trait ReactiveCollectionExt<K, V>: Sized + 'static + ReactiveCollection<K, V>
where
  V: Clone + Send + Sync + 'static,
  K: Send + 'static,
{
  fn into_boxed(self) -> Box<dyn DynamicReactiveCollection<K, V>> {
    Box::new(self)
  }

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
    F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
    V2: Send + Sync + Clone,
    K: Sync + Clone,
    Self: Sync,
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
    F: Fn() -> FF + Send + Sync + 'static,
    FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync,
    V2: Send + Sync + Clone + 'static,
  {
    ReactiveKVExecuteMap {
      inner: self,
      map_creator: f,
      cache: Default::default(),
      phantom: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> impl ReactiveCollection<K, V>
  where
    V: Copy,
    F: Fn(V) -> bool + Copy + Send + Sync + 'static,
    K: Sync + Clone,
    Self: Sync,
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
    F: Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
    V2: Send + Sync + Clone,
    K: Sync + Clone,
    Self: Sync,
  {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  fn collective_union<V2, Other, F, O>(self, other: Other, f: F) -> impl ReactiveCollection<K, O>
  where
    Other: ReactiveCollection<K, V2>,
    K: Copy + std::hash::Hash + Eq + Send + Sync,
    V2: Clone + Send + Sync + 'static,
    O: Send + Sync + Clone + 'static,
    F: Fn((Option<V>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
    Self: Sync,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      phantom: PhantomData,
      f,
    }
  }

  /// K should not overlap
  /// todo impl more efficient version
  fn collective_select<Other>(self, other: Other) -> impl ReactiveCollection<K, V>
  where
    K: Copy + std::hash::Hash + Eq + Send + Sync + 'static,
    Other: ReactiveCollection<K, V>,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(_), Some(_)) => unreachable!("key set should not overlap"),
      (Some(a), None) => a.into(),
      (None, Some(b)) => b.into(),
      (None, None) => unreachable!("value not selected"),
    })
  }

  /// K should fully overlap
  fn collective_zip<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + Send + Sync + 'static,
    Other: ReactiveCollection<K, V2>,
    V2: Clone + Send + Sync + 'static,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => unreachable!("value not zipped"),
    })
  }

  /// only return overlapped part
  fn collective_intersect<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + Send + Sync + 'static,
    Other: ReactiveCollection<K, V2>,
    V2: Clone + Send + Sync + 'static,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => None,
    })
  }

  /// filter map<k, v> by reactive set<k>
  /// have to use box here to avoid complex type(could be improved)
  fn filter_by_keyset<S>(self, set: S) -> impl ReactiveCollection<K, V>
  where
    K: Copy + std::hash::Hash + Eq + Send + Sync + 'static,
    S: ReactiveCollection<K, ()>,
  {
    self.collective_intersect(set).collective_map(|(v, _)| v)
  }

  fn into_forker(self) -> ReactiveKVMapFork<Self, CollectionDelta<K, V>, K, V> {
    ReactiveKVMapFork::new(self)
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    V: Clone + Send + Sync + 'static,
    MK: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    K: Clone + Sync + 'static,
    Relation: ReactiveOneToManyRelationship<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: self,
      relations,
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

  fn into_collection_with_previous(self) -> impl ReactiveCollectionWithPrevious<K, V>
  where
    Self: Sized,
    K: Send + Sync + 'static + Clone + Eq + std::hash::Hash,
    V: Send + Sync + 'static + Clone + PartialEq,
  {
    IntoReactiveCollectionWithPrevious {
      inner: self,
      phantom: Default::default(),
      current: Default::default(),
    }
  }

  fn filter_redundant_remove(self) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + Clone + Send + Sync + Eq + std::hash::Hash + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    ReactiveCollectionMessageFilter {
      inner: self,
      state: Default::default(),
      phantom: PhantomData,
    }
  }

  fn debug(self) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + Clone + Send + Sync + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    ReactiveCollectionDebug {
      inner: self,
      phantom: PhantomData,
    }
  }
}
impl<T, K, V> ReactiveCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveCollection<K, V>,
  V: Clone + Send + Sync + 'static,
  K: Send + 'static,
{
}

struct IntoReactiveCollectionWithPrevious<T, K, V> {
  inner: T,
  phantom: PhantomData<(K, V)>,
  current: FastHashMap<K, V>,
}

impl<T, K, V> ReactiveCollectionWithPrevious<K, V> for IntoReactiveCollectionWithPrevious<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: Send + Sync + Eq + std::hash::Hash + 'static + Clone,
  V: Send + Sync + 'static + Clone + PartialEq,
{
  type Changes = impl CollectionChangesWithPrevious<K, V>;

  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<Self::Changes>> {
    self.inner.poll_changes(cx).map(|v| {
      v.map(|v| {
        let v = v
          .collect_into_pass_vec()
          .vec
          .iter()
          .cloned()
          .collect::<Vec<_>>();

        v.into_par_iter()
          .collect::<Vec<_>>()
          .into_iter()
          .filter_map(|v| match v {
            CollectionDelta::Delta(k, v) => {
              let pre = self.current.insert(k.clone(), v.clone());
              if let Some(pre) = pre {
                if pre != v {
                  CollectionDeltaWithPrevious::Delta(k, v, Some(pre)).into()
                } else {
                  None
                }
              } else {
                CollectionDeltaWithPrevious::Delta(k, v, pre).into()
              }
            }
            CollectionDelta::Remove(k) => {
              if let Some(v) = self.current.remove(&k) {
                CollectionDeltaWithPrevious::Remove(k, v).into()
              } else {
                None
              }
            }
          })
          .collect::<Vec<_>>()
          .into_par_iter()
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, K, V> VirtualCollection<K, V> for IntoReactiveCollectionWithPrevious<T, K, V>
where
  T: VirtualCollection<K, V> + Sync,
  K: Clone + Sync + Eq + std::hash::Hash,
  V: Clone + Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.current.keys().cloned()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    |key| self.current.get(key).cloned()
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
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = Map::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.inner.poll_changes(cx);
    if let Poll::Ready(Some(changes)) = &r {
      let changes = changes.clone().collect_into_pass_vec();
      for change in changes {
        match change {
          CollectionDelta::Delta(k, v) => {
            self.cache.insert(k, v);
          }
          CollectionDelta::Remove(k) => {
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
  Map: VirtualCollection<K, V> + Sync,
  K: std::hash::Hash + Eq + Clone + Sync,
  V: Clone + Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.cache.keys().cloned()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    move |key| self.cache.get(key).cloned()
  }
}

pub struct LinearMaterializedReactiveCollection<Map, V> {
  inner: Map,
  cache: IndexKeptVec<V>,
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  Map::Changes: Clone,
  K: LinearIdentification + Send + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = Map::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.inner.poll_changes(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().collect_into_pass_vec() {
        match change {
          CollectionDelta::Delta(k, v) => {
            self.cache.insert(v, k.alloc_index());
          }
          CollectionDelta::Remove(k) => {
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
  Map: VirtualCollection<K, V> + Sync,
  K: LinearIdentification + 'static,
  V: Sync + Clone,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.cache.iter().map(|(k, _)| K::from_alloc_index(k))
  }
  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    move |key| self.cache.try_get(key.alloc_index()).cloned()
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T, F, K, V, V2> {
  inner: T,
  map_creator: F,
  cache: DashMap<K, V2, FastHasherBuilder>,
  phantom: PhantomData<(K, V, V2)>,
}

impl<T, F, K, V, V2, FF> ReactiveCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V, V2>
where
  V: Sync + Send + 'static,
  K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
  F: Fn() -> FF + Send + Sync + 'static,
  FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
  V2: Clone + Send + Sync + 'static,
  T: ReactiveCollection<K, V>,
{
  type Changes = impl CollectionChanges<K, V2>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        let mapper = (self.map_creator)();
        deltas
          .map(move |delta| match delta {
            CollectionDelta::Delta(k, d) => {
              let new_value = mapper(&k, d);
              self.cache.insert(k.clone(), new_value.clone());
              CollectionDelta::Delta(k, new_value)
            }
            CollectionDelta::Remove(k) => {
              self.cache.remove(&k);
              CollectionDelta::Remove(k)
            }
          })
          .collect_into_pass_vec()
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.shrink_to_fit(),
    }
    self.inner.extra_request(request)
  }
}

impl<T, F, FF, K, V, V2> VirtualCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V, V2>
where
  F: Fn() -> FF + Sync + 'static,
  FF: Fn(&K, V) -> V2 + Sync + 'static,
  T: VirtualCollection<K, V> + Sync,
  K: Eq + std::hash::Hash + Sync + Send + Clone,
  V: Sync,
  V2: Clone + Sync + Send,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.cache.iter().map(|e| e.key().clone())
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + Sync + '_ {
    move |key| self.cache.get(key).map(|v| v.value().clone())
  }
}

pub struct ReactiveKVMap<T, F, K, V> {
  inner: T,
  map: F,
  phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  V: Send + Sync + Clone + 'static,
  K: Send + Sync + Clone + 'static,
  V2: Send + Sync + Clone,
  F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V> + Sync,
{
  type Changes = impl CollectionChanges<K, V2>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let mapper = self.map;
    self.inner.poll_changes(cx).map(move |r| {
      r.map(move |deltas| {
        deltas
          .into_par_iter()
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
  F: Fn(V) -> V2 + Sync + Copy,
  K: Sync,
  T: VirtualCollection<K, V> + Sync,
  V: Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + Sync + '_ {
    let inner_getter = self.inner.access();
    move |key| inner_getter(key).map(|v| (self.map)(v))
  }
}

pub struct ReactiveCollectionMessageFilter<T, K, V> {
  inner: T,
  state: ActivationState<K>,
  phantom: PhantomData<(K, V)>,
}

impl<T, K, V> VirtualCollection<K, V> for ReactiveCollectionMessageFilter<T, K, V>
where
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    self.inner.access()
  }
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionMessageFilter<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: std::fmt::Debug + Clone + Send + Sync + Eq + std::hash::Hash + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChanges<K, V>;

  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<Self::Changes>> {
    let changes = self
      .inner
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()));

    if let Poll::Ready(Some(changes)) = &changes {
      let mut filtered = Vec::with_capacity(changes.vec.len());
      for change in changes.vec.as_slice() {
        if self.state.update(change) {
          filtered.push(change.clone())
        }
      }
      if filtered.is_empty() {
        return Poll::Pending;
      } else {
        return Poll::Ready(Some(FastPassingVec::from_vec(filtered)));
      }
    }

    changes
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.state.inner.shrink_to_fit(),
    }
  }
}

pub struct ReactiveCollectionDebug<T, K, V> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<T, K, V> ReactiveCollectionWithPrevious<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollectionWithPrevious<K, V>,
  K: std::fmt::Debug + Clone + Send + Sync + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChangesWithPrevious<K, V>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self
      .inner
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()));
    if let Poll::Ready(Some(v)) = &r {
      println!("{:#?}", v);
    }
    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: std::fmt::Debug + Clone + Send + Sync + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChanges<K, V>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self
      .inner
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()));
    if let Poll::Ready(Some(v)) = &r {
      println!("{:#?}", v);
    }
    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, K, V> VirtualCollection<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    self.inner.access()
  }
}

pub struct ReactiveKVFilter<T, F, K, V> {
  inner: T,
  checker: F,
  k: PhantomData<(K, V)>,
}

fn make_checker<K, V, V2>(
  checker: impl Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
) -> impl Fn(CollectionDelta<K, V>) -> CollectionDelta<K, V2> + Copy + Send + Sync + 'static {
  move |delta| {
    match delta {
      CollectionDelta::Delta(k, v) => {
        if let Some(new_v) = checker(v) {
          CollectionDelta::Delta(k, new_v)
        } else {
          CollectionDelta::Remove(k)
        }
      }
      // the Remove variant maybe called many times for given k
      CollectionDelta::Remove(k) => CollectionDelta::Remove(k),
    }
  }
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V> + Sync,
  K: Send + Sync + Clone + 'static,
  V: Send + Sync + Clone + 'static,
  V2: Send + Sync + Clone,
{
  type Changes = impl CollectionChanges<K, V2>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let checker = make_checker(self.checker);
    self
      .inner
      .poll_changes(cx)
      .map(move |r| r.map(move |deltas| deltas.into_par_iter().map(checker)))
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, F, K, V, V2> VirtualCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Sync + Copy,
  T: VirtualCollection<K, V> + Sync,
  K: Sync,
  V: Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    let inner_getter = self.inner.access();
    self
      .inner
      .iter_key()
      .filter(move |k| inner_getter(k).and_then(|v| (self.checker)(v)).is_some())
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + Sync + '_ {
    let inner_getter = self.inner.access();
    move |key| inner_getter(key).and_then(|v| (self.checker)(v))
  }
}

pub struct ReactiveKVUnion<T1, T2, K, F, O, V1, V2> {
  a: T1,
  b: T2,
  phantom: PhantomData<(K, O, V1, V2)>,
  f: F,
}

impl<T1, T2, K, V1, V2, F, O> VirtualCollection<K, O> for ReactiveKVUnion<T1, T2, K, F, O, V1, V2>
where
  K: Copy + std::hash::Hash + Eq + Sync,
  T1: VirtualCollection<K, V1> + Sync,
  T2: VirtualCollection<K, V2> + Sync,
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + 'static,
  O: Sync,
  V1: Sync,
  V2: Sync,
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
  fn access(&self) -> impl Fn(&K) -> Option<O> + Sync + '_ {
    let getter_a = self.a.access();
    let getter_b = self.b.access();

    move |key| {
      let (v1, v2) = (getter_a(key), getter_b(key));
      if v1.is_none() && v2.is_none() {
        None
      } else {
        (self.f)((v1, v2))
      }
    }
  }
}

/// we should manually impl zip, intersect, select, to avoid overhead
fn union<K: Clone, V1, V2>(
  change1: Option<CollectionDelta<K, V1>>,
  change2: Option<CollectionDelta<K, V2>>,
  v1_current: &impl Fn(&K) -> Option<V1>,
  v2_current: &impl Fn(&K) -> Option<V2>,
) -> Option<CollectionDelta<K, (Option<V1>, Option<V2>)>> {
  let r = match (change1, change2) {
    (None, None) => return None,
    (None, Some(change2)) => match change2 {
      CollectionDelta::Delta(k, v2) => {
        let v1_current = v1_current(&k);
        CollectionDelta::Delta(k, (v1_current, Some(v2)))
      }
      CollectionDelta::Remove(k) => {
        if let Some(v1_current) = v1_current(&k) {
          CollectionDelta::Delta(k, (Some(v1_current), None))
        } else {
          CollectionDelta::Remove(k)
        }
      }
    },
    (Some(change1), None) => match change1 {
      CollectionDelta::Delta(k, v1) => {
        let v2_current = v2_current(&k);
        CollectionDelta::Delta(k, (Some(v1), v2_current))
      }
      CollectionDelta::Remove(k) => {
        if let Some(v2_current) = v2_current(&k) {
          CollectionDelta::Delta(k, (None, Some(v2_current)))
        } else {
          CollectionDelta::Remove(k)
        }
      }
    },
    (Some(change1), Some(change2)) => match (change1, change2) {
      (CollectionDelta::Delta(k, v1), CollectionDelta::Delta(_, v2)) => {
        CollectionDelta::Delta(k, (Some(v1), Some(v2)))
      }
      (CollectionDelta::Delta(_, v1), CollectionDelta::Remove(k)) => {
        CollectionDelta::Delta(k.clone(), (Some(v1), v2_current(&k)))
      }
      (CollectionDelta::Remove(k), CollectionDelta::Delta(_, v2)) => {
        CollectionDelta::Delta(k.clone(), (v1_current(&k), Some(v2)))
      }
      (CollectionDelta::Remove(k), CollectionDelta::Remove(_)) => CollectionDelta::Remove(k),
    },
  };

  if let CollectionDelta::Delta(k, new) = r {
    return CollectionDelta::Delta(k, new).into();
  }

  r.into()
}

impl<T1, T2, K, V1, V2, F, O> ReactiveCollection<K, O> for ReactiveKVUnion<T1, T2, K, F, O, V1, V2>
where
  K: Copy + std::hash::Hash + Eq + Send + Sync + 'static,
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  O: Send + Sync + Clone + 'static,
  T1: ReactiveCollection<K, V1>,
  T2: ReactiveCollection<K, V2>,
  V1: Clone + Send + Sync + 'static,
  V2: Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChanges<K, O>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let t1 = self.a.poll_changes(cx);
    let t2 = self.b.poll_changes(cx);

    let a_access = self.a.access();
    let b_access = self.b.access();

    let checker = make_checker(self.f);

    let r = match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let mut intersections: FastHashMap<
          K,
          (
            Option<CollectionDelta<K, V1>>,
            Option<CollectionDelta<K, V2>>,
          ),
        > = FastHashMap::default();
        v1.collect_into_pass_vec().into_iter().for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().0 = Some(d)
        });

        v2.collect_into_pass_vec().into_iter().for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().1 = Some(d)
        });

        intersections
          .into_par_iter()
          .filter_map(|(_, (d1, d2))| union(d1, d2, &a_access, &b_access))
          .map(checker)
          .collect::<Vec<_>>()
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => v1
        .filter_map(|d1| union(Some(d1), None, &a_access, &b_access))
        .map(checker)
        .collect::<Vec<_>>(),
      (Poll::Pending, Poll::Ready(Some(v2))) => v2
        .filter_map(|d2| union(None, Some(d2), &a_access, &b_access))
        .map(checker)
        .collect::<Vec<_>>(),

      (Poll::Pending, Poll::Pending) => return Poll::Pending,
      _ => return Poll::Ready(None),
    };

    if r.is_empty() {
      return Poll::Pending;
    }

    Poll::Ready(Some(r.into_par_iter()))
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.a.extra_request(request);
    self.b.extra_request(request);
  }
}

// /// when we want to zip multiple kv, using deeply nested zipper is viable, however it's
// computation /// intensive during layer of layers consuming. This combinator provides the
// flattened version of /// multi zip, in trades of the performance overhead of dynamical fn call
// and internal cache(maybe /// user still require this so it's ok).
// pub struct MultiZipper<K, P, V> {
//   sources: Vec<Box<dyn DynamicReactiveCollection<K, P> + Sync>>,
//   current: FastHashMap<K, V>,
//   defaulter: Box<dyn Fn() -> V + Sync + Send>,
//   applier: Box<dyn Fn(&mut V, P) + Sync + Send>,
// }

// impl<K, P, V> MultiZipper<K, P, V> {
//   pub fn new(
//     defaulter: impl Fn() -> V + Sync + Send + 'static,
//     applier: impl Fn(&mut V, P) + Sync + Send + 'static,
//   ) -> Self {
//     Self {
//       sources: Default::default(),
//       current: Default::default(),
//       defaulter: Box::new(defaulter),
//       applier: Box::new(applier),
//     }
//   }

//   pub fn zip_with(mut self, source: impl ReactiveCollection<K, P> + Sync) -> Self
//   where
//     K: Send + 'static,
//     P: Send + 'static,
//   {
//     self.sources.push(Box::new(source));
//     self
//   }
// }

// impl<K, P, V> VirtualCollection<K, V> for MultiZipper<K, P, V>
// where
//   K: Clone + Eq + std::hash::Hash + Sync,
//   V: Clone + Sync,
// {
//   fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
//     self.current.keys().cloned()
//   }

//   fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
//     |k| self.current.get(k).cloned()
//   }
// }

// impl<K, P, V> ReactiveCollection<K, V> for MultiZipper<K, P, V>
// where
//   K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
//   V: Clone + Send + Sync + 'static,
//   P: Clone + 'static,
// {
//   type Changes = impl CollectionChanges<K, V>;

//   #[allow(clippy::collapsible_else_if)]
//   fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
//     if self.sources.is_empty() {
//       return Poll::Pending;
//     }

//     let mut outputs = FastHashMap::<K, CollectionDelta<K, V>>::default();

//     for source in &mut self.sources {
//       if let Poll::Ready(Some(source)) = source.poll_changes_dyn(cx) {
//         for change in source {
//           match change {
//             CollectionDelta::Delta(key, change, ) => {
//               if let Some(previous_new) = outputs.remove(&key) {
//                 match previous_new {
//                   CollectionDelta::Delta(_, mut next, ) => {
//                     (self.applier)(&mut next, change);
//                     outputs.insert(key.clone(), CollectionDelta::Delta(key.clone(), next, pre));
//                   }
//                   CollectionDelta::Remove(_, _) => unreachable!("unexpected zipper input"),
//                 }
//                 outputs.insert(
//                   key.clone(),
//                   CollectionDelta::Delta(key, (self.defaulter)(), None),
//                 );
//               } else {
//                 if let Some(current) = self.current.get(&key) {
//                   let mut next = current.clone();
//                   (self.applier)(&mut next, change);
//                   outputs.insert(
//                     key.clone(),
//                     CollectionDelta::Delta(key, next, Some(current.clone())),
//                   );
//                 } else {
//                   let mut next = (self.defaulter)();
//                   (self.applier)(&mut next, change);
//                   outputs.insert(key.clone(), CollectionDelta::Delta(key, next, None));
//                 }
//               }
//             }
//             CollectionDelta::Remove(key, _) => {
//               outputs.insert(
//                 key.clone(),
//                 CollectionDelta::Remove(key.clone(), self.current.remove(&key).unwrap()),
//               );
//             }
//           }
//         }
//       }
//     }

//     if outputs.is_empty() {
//       return Poll::Pending;
//     }

//     for v in outputs.values() {
//       match v {
//         CollectionDelta::Delta(k, next, _) => {
//           self.current.insert(k.clone(), next.clone());
//         }
//         CollectionDelta::Remove(k, _) => {
//           self.current.remove(k);
//         }
//       }
//     }

//     Poll::Ready(Some(HashMapIntoValue::new(outputs)))
//   }

//   fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
//     self
//       .sources
//       .iter_mut()
//       .for_each(|s| s.extra_request_dyn(request));
//     match request {
//       ExtraCollectionOperation::MemoryShrinkToFit => self.current.shrink_to_fit(),
//     }
//   }
// }

/// workaround hashmap parallel iterator not impl clone

pub(crate) struct HashMapIntoValue<K, V> {
  map: FastHashMap<K, V>,
}

impl<K, V> HashMapIntoValue<K, V> {
  pub fn new(map: FastHashMap<K, V>) -> Self {
    Self { map }
  }
}

impl<K: Clone, V: Clone> Clone for HashMapIntoValue<K, V> {
  fn clone(&self) -> Self {
    Self {
      map: self.map.clone(),
    }
  }
}

impl<K, V> ParallelIterator for HashMapIntoValue<K, V>
where
  K: Send + Eq + std::hash::Hash,
  V: Send,
{
  type Item = V;

  fn drive_unindexed<C>(self, consumer: C) -> C::Result
  where
    C: ::rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
  {
    self
      .map
      .into_par_iter()
      .map(|(_, v)| v)
      .drive_unindexed(consumer)
  }
}
