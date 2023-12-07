use std::{marker::PhantomData, ops::DerefMut};

use fast_hash_collection::*;
use storage::IndexKeptVec;

use crate::*;

#[derive(Clone, Copy)]
pub enum CPoll<T> {
  Ready(T),
  Pending,
  Blocked,
}

impl<T> CPoll<T> {
  pub fn is_blocked(&self) -> bool {
    matches!(self, CPoll::Blocked)
  }
  pub fn is_pending(&self) -> bool {
    matches!(self, CPoll::Pending)
  }
  pub fn map<T2>(self, f: impl FnOnce(T) -> T2) -> CPoll<T2> {
    match self {
      CPoll::Ready(v) => CPoll::Ready(f(v)),
      CPoll::Pending => CPoll::Pending,
      CPoll::Blocked => CPoll::Blocked,
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  // k, new_v
  Delta(K, V),
  // k
  Remove(K),
}

impl<K, V> CollectionDelta<K, V> {
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

  pub fn is_removed(&self) -> bool {
    match self {
      Self::Remove(_) => true,
      Self::Delta(_, _) => false,
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

  // todo, currently we not enforce that the access should be match the iter_key result so
  // required to handle None case
  fn iter_key_value_forgive(&self) -> impl Iterator<Item = (K, V)> + '_ {
    let access = self.access();
    self
      .iter_key()
      .filter_map(move |k| access(&k).map(|v| (k, v)))
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

  // todo, remove box
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>>;
}

pub trait VirtualMultiCollection<K, V> {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_;
  fn access_multi(&self) -> impl Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_;
  fn try_access_multi(&self) -> Option<Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>>;
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
pub trait ReactiveCollection<K: Send, V: Send>:
  VirtualCollection<K, V> + Sync + Send + 'static
{
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<CollectionChanges<K, V>>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn spin_poll_until_pending(
    &mut self,
    cx: &mut Context,
    mut consumer: impl FnMut(CollectionChanges<K, V>),
  ) {
    loop {
      match self.poll_changes(cx) {
        CPoll::Ready(r) => consumer(r),
        CPoll::Pending => return,
        CPoll::Blocked => continue,
      }
    }
  }
}

pub type CollectionChanges<K, V> = FastHashMap<K, CollectionDelta<K, V>>;

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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}

impl<K, V> ReactiveCollection<K, V> for ()
where
  K: 'static + Send + Sync + Clone,
  V: 'static + Send + Sync + Clone,
{
  fn poll_changes(&mut self, _: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    CPoll::Pending
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}
impl<K, V> ReactiveCollection<K, V> for ConstCollection<V>
where
  K: 'static + Send + Sync + Clone,
  V: 'static + Send + Sync + Clone,
{
  fn poll_changes(&mut self, _: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    CPoll::Pending
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}

/// dynamic version of the above trait
pub trait DynamicVirtualCollection<K, V> {
  fn iter_key_boxed(&self) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_boxed(&self) -> Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
  fn try_access_boxed(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>>;
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
  fn try_access_boxed(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.try_access()
  }
}

pub trait DynamicReactiveCollection<K, V>: DynamicVirtualCollection<K, V> + Sync + Send {
  fn poll_changes_dyn(&mut self, _cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>>;
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K, V, T> DynamicReactiveCollection<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: Send + 'static,
  V: Send + 'static,
{
  fn poll_changes_dyn(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    self.poll_changes(cx)
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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.deref().try_access_boxed()
  }
}
impl<K, V> ReactiveCollection<K, V> for Box<dyn DynamicReactiveCollection<K, V>>
where
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
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
  type Item = CollectionChanges<K, V>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    match this.inner.poll_changes(cx) {
      CPoll::Ready(r) => Poll::Ready(Some(r)),
      CPoll::Pending => Poll::Pending,
      CPoll::Blocked => Poll::Pending, // this is logically ok
    }
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

  fn into_change_stream(self) -> impl Stream<Item = CollectionChanges<K, V>>
  where
    Self: Unpin,
  {
    ReactiveCollectionAsStream {
      inner: self,
      phantom: PhantomData,
    }
  }

  #[inline(always)]
  fn workaround_box(self) -> impl ReactiveCollection<K, V>
  where
    K: Clone + Sync,
  {
    let r = self;
    // this is a workaround that the compiler maybe generate huge outputs(like pdb file)  which lead
    // to link error in debug build, as well as using huge memory
    // see https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
    #[cfg(debug_assertions)]
    let r = r.into_boxed();

    r
  }

  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
    V2: Send + Sync + Clone + 'static,
    K: Sync + Clone + Eq + std::hash::Hash,
    Self: Sync,
  {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
    .workaround_box()
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
    .workaround_box()
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> impl ReactiveCollection<K, V>
  where
    V: Copy,
    F: Fn(V) -> bool + Copy + Send + Sync + 'static,
    K: Sync + Clone + Eq + std::hash::Hash,
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
    V2: Send + Sync + Clone + 'static,
    K: Sync + Clone + 'static + Eq + std::hash::Hash,
    Self: Sync,
  {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
    .workaround_box()
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
      a: BufferedCollection::new(self),
      b: BufferedCollection::new(other),
      phantom: PhantomData,
      f,
    }
    .workaround_box()
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
    self.collective_union(set, |(a, b)| match (a, b) {
      (Some(a), Some(_)) => Some(a),
      _ => None,
    })
  }

  fn into_forker(self) -> ReactiveKVMapFork<Self, CollectionChanges<K, V>, K, V> {
    BufferedCollection::new(ReactiveKVMapForkImpl::new(self))
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    V: Clone + Send + Sync + 'static,
    MK: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    K: Clone + Eq + std::hash::Hash + Sync + 'static,
    Relation: ReactiveOneToManyRelationship<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: BufferedCollection::new(self),
      relations: BufferedCollection::new(relations),
      phantom: PhantomData,
    }
    .workaround_box()
    .filter_redundant_remove()
  }

  fn materialize_unordered(self) -> impl ReactiveCollection<K, V>
  where
    K: Eq + std::hash::Hash + Clone + Sync,
  {
    UnorderedMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
    .workaround_box()
  }
  fn materialize_linear(self) -> impl ReactiveCollection<K, V>
  where
    K: LinearIdentification + Sync,
  {
    LinearMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
    .workaround_box()
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

  // this maybe helpful to performance and has small memory overhead
  fn filter_redundant_remove(self) -> impl ReactiveCollection<K, V>
  where
    K: Clone + Send + Sync + Eq + std::hash::Hash + 'static,
    V: Clone + Send + Sync + 'static,
  {
    ReactiveCollectionMessageFilter {
      inner: self,
      state: Default::default(),
      phantom: PhantomData,
    }
    .workaround_box()
  }

  fn debug(self, label: &'static str) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + Clone + Send + Sync + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    ReactiveCollectionDebug {
      inner: self,
      phantom: PhantomData,
      label,
    }
    .workaround_box()
  }

  fn debug_using_net_change(self, label: &'static str) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + PartialEq + 'static,
  {
    self
      .into_collection_with_previous()
      .debug(label)
      .into_collection()
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
  #[tracing::instrument(skip_all, name = "IntoReactiveCollectionWithPrevious")]
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<CollectionChangesWithPrevious<K, V>> {
    let mut is_empty = false;
    let r = self.inner.poll_changes(cx).map(|v| {
      let output: CollectionChangesWithPrevious<K, V> = v
        .into_values()
        .filter_map(|v| match v {
          CollectionDelta::Delta(k, v) => {
            let pre = self.current.insert(k.clone(), v.clone());
            if let Some(pre) = pre {
              if pre != v {
                (
                  k.clone(),
                  CollectionDeltaWithPrevious::Delta(k, v, Some(pre)),
                )
                  .into()
              } else {
                None
              }
            } else {
              (k.clone(), CollectionDeltaWithPrevious::Delta(k, v, pre)).into()
            }
          }
          CollectionDelta::Remove(k) => {
            if let Some(v) = self.current.remove(&k) {
              (k.clone(), CollectionDeltaWithPrevious::Remove(k, v)).into()
            } else {
              None
            }
          }
        })
        .collect();

      if output.is_empty() {
        is_empty = true;
      }

      output
    });

    if is_empty {
      return CPoll::Pending;
    }

    r
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}

pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  inner: Map,
  cache: FastHashMap<K, V>,
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    let r = self.inner.poll_changes(cx);
    if let CPoll::Ready(changes) = &r {
      for change in changes.values() {
        match change.clone() {
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}

pub struct LinearMaterializedReactiveCollection<Map, V> {
  inner: Map,
  cache: IndexKeptVec<V>,
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  K: LinearIdentification + Send + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    let r = self.inner.poll_changes(cx);
    if let CPoll::Ready(changes) = &r {
      for change in changes.values().cloned() {
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T, F, K, V, V2> {
  inner: T,
  map_creator: F,
  cache: FastHashMap<K, V2>,
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
  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V2>> {
    self.inner.poll_changes(cx).map(move |deltas| {
      let mapper = (self.map_creator)();
      deltas
        .into_values()
        .map(|delta| match delta {
          CollectionDelta::Delta(k, d) => {
            let new_value = mapper(&k, d);
            self.cache.insert(k.clone(), new_value.clone());
            (k.clone(), CollectionDelta::Delta(k, new_value))
          }
          CollectionDelta::Remove(k) => {
            self.cache.remove(&k);
            (k.clone(), CollectionDelta::Remove(k))
          }
        })
        .collect()
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
    self.cache.keys().cloned()
  }
  fn access(&self) -> impl Fn(&K) -> Option<V2> + Sync + '_ {
    move |key| self.cache.get(key).cloned()
  }
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V2> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V2> + Sync + '_>;
    boxed.into()
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
  K: Send + Sync + Clone + Eq + std::hash::Hash + 'static,
  V2: Send + Sync + Clone,
  F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V> + Sync,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVMap")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V2>> {
    let mapper = self.map;
    self.inner.poll_changes(cx).map(move |deltas| {
      deltas
        .into_iter()
        .map(move |(k, delta)| (k, delta.map(|_, v| mapper(v))))
        .collect()
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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V2> + Sync + '_>> {
    let inner_getter = self.inner.try_access()?;
    let boxed = Box::new(move |key: &_| inner_getter(key).map(|v| (self.map)(v)))
      as Box<dyn for<'a> Fn(&'a K) -> Option<V2> + Sync + '_>;
    boxed.into()
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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.inner.try_access()
  }
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionMessageFilter<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: Clone + Send + Sync + Eq + std::hash::Hash + 'static,
  V: Clone + Send + Sync + 'static,
{
  #[tracing::instrument(skip_all, name = "ReactiveCollectionMessageFilter")]
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<CollectionChanges<K, V>> {
    let changes = self.inner.poll_changes(cx);

    if let CPoll::Ready(changes) = changes {
      let filtered: CollectionChanges<K, V> = changes
        .into_iter()
        .filter(|(_, v)| self.state.update(v))
        .collect();
      if filtered.is_empty() {
        CPoll::Pending
      } else {
        CPoll::Ready(filtered)
      }
    } else {
      changes
    }
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
  pub label: &'static str,
}

impl<T, K, V> ReactiveCollectionWithPrevious<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollectionWithPrevious<K, V>,
  K: std::fmt::Debug + Clone + Send + Sync + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChangesWithPrevious<K, V>> {
    let r = self.inner.poll_changes(cx);
    if let CPoll::Ready(v) = &r {
      println!("{} {:#?}", self.label, v);
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
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    let r = self.inner.poll_changes(cx);
    if let CPoll::Ready(v) = &r {
      let v = v.values().collect::<Vec<_>>();
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.inner.try_access()
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
  K: Send + Sync + Clone + Eq + std::hash::Hash + 'static,
  V: Send + Sync + Clone + 'static,
  V2: Send + Sync + Clone,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVFilter")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V2>> {
    let checker = make_checker(self.checker);
    self
      .inner
      .poll_changes(cx)
      .map(move |r| r.into_iter().map(|(k, v)| (k, checker(v))).collect())
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V2> + Sync + '_>> {
    let inner_getter = self.inner.try_access()?;
    let getter = move |key: &_| inner_getter(key).and_then(|v| (self.checker)(v));
    let boxed: Box<dyn Fn(&K) -> Option<V2> + Sync + '_> = Box::new(getter);
    boxed.into()
  }
}

pub struct ReactiveKVUnion<T1, T2, K, F, O, V1, V2> {
  a: BufferedCollection<CollectionChanges<K, V1>, T1>,
  b: BufferedCollection<CollectionChanges<K, V2>, T2>,
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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<O> + Sync + '_>> {
    let getter_a = self.a.try_access()?;
    let getter_b = self.b.try_access()?;

    let acc = move |key: &_| {
      let (v1, v2) = (getter_a(key), getter_b(key));
      if v1.is_none() && v2.is_none() {
        None
      } else {
        (self.f)((v1, v2))
      }
    };
    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<O> + Sync + '_>;
    boxed.into()
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
  #[tracing::instrument(skip_all, name = "ReactiveKVUnion")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, O>> {
    let waker = cx.waker().clone();
    let (t1, t2) = rayon::join(
      || {
        let mut cx = Context::from_waker(&waker);
        self.a.poll_changes(&mut cx)
      },
      || {
        let mut cx = Context::from_waker(&waker);
        self.b.poll_changes(&mut cx)
      },
    );

    let a_access = self.a.try_access();
    let b_access = self.b.try_access();

    if a_access.is_none() || b_access.is_none() {
      drop(a_access);
      drop(b_access);
      if let CPoll::Ready(v) = t1 {
        self.a.put_back_to_buffered(v);
      }
      if let CPoll::Ready(v) = t2 {
        self.b.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    };
    let a_access = a_access.unwrap();
    let b_access = b_access.unwrap();
    let checker = make_checker(self.f);

    let r = match (t1, t2) {
      (CPoll::Ready(v1), CPoll::Ready(v2)) => {
        let mut intersections: FastHashMap<
          K,
          (
            Option<CollectionDelta<K, V1>>,
            Option<CollectionDelta<K, V2>>,
          ),
        > = FastHashMap::default();
        v1.into_values().for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().0 = Some(d)
        });

        v2.into_values().for_each(|d| {
          let key = *d.key();
          intersections.entry(key).or_default().1 = Some(d)
        });

        intersections
          .into_iter()
          .filter_map(|(k, (d1, d2))| {
            union(d1, d2, &a_access, &b_access)
              .map(checker)
              .map(|v| (k, v))
          })
          .collect::<CollectionChanges<K, O>>()
      }
      (CPoll::Ready(v1), CPoll::Pending) => v1
        .into_iter()
        .filter_map(|(k, d1)| {
          union(Some(d1), None, &a_access, &b_access)
            .map(checker)
            .map(|v| (k, v))
        })
        .collect::<CollectionChanges<K, O>>(),
      (CPoll::Pending, CPoll::Ready(v2)) => v2
        .into_iter()
        .filter_map(|(k, d2)| {
          union(None, Some(d2), &a_access, &b_access)
            .map(checker)
            .map(|v| (k, v))
        })
        .collect::<CollectionChanges<K, O>>(),

      (CPoll::Ready(v), CPoll::Blocked) => {
        drop(a_access);
        self.a.put_back_to_buffered(v);
        return CPoll::Blocked;
      }
      (CPoll::Blocked, CPoll::Ready(v)) => {
        drop(b_access);
        self.b.put_back_to_buffered(v);
        return CPoll::Blocked;
      }
      (CPoll::Pending, CPoll::Pending) => return CPoll::Pending,
      (CPoll::Pending, CPoll::Blocked) => return CPoll::Blocked,
      (CPoll::Blocked, CPoll::Pending) => return CPoll::Blocked,
      (CPoll::Blocked, CPoll::Blocked) => return CPoll::Blocked,
    };

    if r.is_empty() {
      return CPoll::Pending;
    }

    CPoll::Ready(r)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.a.extra_request(request);
    self.b.extra_request(request);
  }
}
