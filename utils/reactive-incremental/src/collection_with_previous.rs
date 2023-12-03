use std::{marker::PhantomData, ops::DerefMut};

use fast_hash_collection::*;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDeltaWithPrevious<K, V> {
  // k, new_v, pre_v
  Delta(K, V, Option<V>),
  // k, pre_v
  Remove(K, V),
}

pub trait CollectionChangesWithPrevious<K: Send, V: Send>:
  ParallelIterator<Item = CollectionDeltaWithPrevious<K, V>>
  + MaybeFastCollect<CollectionDeltaWithPrevious<K, V>>
  + Clone
  + Send
  + Sync
  + Sized
{
}
impl<K, V, T> CollectionChangesWithPrevious<K, V> for T
where
  K: Send,
  V: Send,
  T: ParallelIterator<Item = CollectionDeltaWithPrevious<K, V>> + Send + Sync,
  T: MaybeFastCollect<CollectionDeltaWithPrevious<K, V>>,
  T: Clone,
{
}

pub trait ReactiveCollectionWithPrevious<K: Send, V: Send>:
  VirtualCollection<K, V> + Sync + Send + 'static
{
  type Changes: CollectionChangesWithPrevious<K, V>;
  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<Self::Changes>>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn poll_changes_and_merge_until_pending(
    &mut self,
    cx: &mut Context,
  ) -> Poll<Option<FastPassingVec<CollectionDeltaWithPrevious<K, V>>>>
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
        deduplicate_collection_changes_previous(&mut hash, v.vec.as_slice().iter().cloned());
      }
      deduplicate_collection_changes_previous(&mut hash, r.vec.as_slice().iter().cloned());

      while let Poll::Ready(Some(v)) = self.poll_changes(cx) {
        let v = v.collect_into_pass_vec();
        deduplicate_collection_changes_previous(&mut hash, v.vec.as_slice().iter().cloned());
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

  fn into_forker(self) -> ReactiveKVMapFork<Self, CollectionDeltaWithPrevious<K, V>, K, V>
  where
    Self: Sized,
  {
    ReactiveKVMapFork::new(self)
  }

  fn debug(self, label: &'static str) -> impl ReactiveCollectionWithPrevious<K, V>
  where
    Self: Sized,
    K: std::fmt::Debug + Clone + Send + Sync + 'static,
    V: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    ReactiveCollectionDebug {
      inner: self,
      phantom: PhantomData,
      label,
    }
  }

  fn into_collection(self) -> impl ReactiveCollection<K, V>
  where
    Self: Sized,
    K: Send + Sync + 'static + Clone,
    V: Send + Sync + 'static + Clone,
  {
    IntoReactiveCollection {
      inner: self,
      phantom: Default::default(),
    }
  }
}

pub trait DynamicReactiveCollectionWithPrevious<K, V>:
  DynamicVirtualCollection<K, V> + Sync + Send
{
  fn poll_changes_dyn(
    &mut self,
    _cx: &mut Context<'_>,
  ) -> Poll<Option<FastPassingVec<CollectionDeltaWithPrevious<K, V>>>>;
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K, V, T> DynamicReactiveCollectionWithPrevious<K, V> for T
where
  T: ReactiveCollectionWithPrevious<K, V>,
  K: Send + 'static,
  V: Send + 'static,
{
  fn poll_changes_dyn(
    &mut self,
    cx: &mut Context<'_>,
  ) -> Poll<Option<FastPassingVec<CollectionDeltaWithPrevious<K, V>>>> {
    self
      .poll_changes(cx)
      .map(|v| v.map(|v| v.collect_into_pass_vec()))
  }
  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<K, V> VirtualCollection<K, V> for Box<dyn DynamicReactiveCollectionWithPrevious<K, V>> {
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.deref().iter_key_boxed()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    self.deref().access_boxed()
  }
}
impl<K, V> ReactiveCollectionWithPrevious<K, V>
  for Box<dyn DynamicReactiveCollectionWithPrevious<K, V>>
where
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = impl CollectionChangesWithPrevious<K, V>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.deref_mut().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}

pub fn deduplicate_collection_changes_previous<K, V>(
  deduplicate: &mut FastHashMap<K, CollectionDeltaWithPrevious<K, V>>,
  deltas: impl Iterator<Item = CollectionDeltaWithPrevious<K, V>>,
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

impl<K, V> CollectionDeltaWithPrevious<K, V> {
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k, _) => k,
      Self::Delta(k, _, _) => k,
    }
  }

  pub fn merge(self, later: Self) -> Option<Self>
  where
    K: Eq,
  {
    use CollectionDeltaWithPrevious::*;
    if self.key() != later.key() {
      panic!("only same key change could be merge");
    }
    match (self, later) {
      (Delta(k, _d1, p1), Delta(_, d2, _p2)) => {
        // we should check d1 = d2
        Delta(k, d2, p1)
      }
      (Delta(k, _d1, p1), Remove(_, _p2)) => {
        // we should check d1 = d2
        if let Some(p1) = p1 {
          Remove(k, p1)
        } else {
          return None;
        }
      }
      (Remove(k, _), Delta(_, d1, p2)) => {
        assert!(p2.is_none());
        Delta(k, d1, None)
      }
      (Remove(_, _), Remove(_, _)) => {
        unreachable!("same key with double remove is invalid")
      }
    }
    .into()
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
}

struct IntoReactiveCollection<T, K, V> {
  inner: T,
  phantom: PhantomData<(K, V)>,
}

impl<T, K, V> ReactiveCollection<K, V> for IntoReactiveCollection<T, K, V>
where
  T: ReactiveCollectionWithPrevious<K, V>,
  K: Send + Sync + 'static + Clone,
  V: Send + Sync + 'static + Clone,
{
  type Changes = impl CollectionChanges<K, V>;

  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<Self::Changes>> {
    self.inner.poll_changes(cx).map(|v| {
      v.map(|v| {
        let v = v
          .collect_into_pass_vec()
          .vec
          .iter()
          .cloned()
          .collect::<Vec<_>>();
        v.into_par_iter().map(|v| match v {
          CollectionDeltaWithPrevious::Delta(k, v, _) => CollectionDelta::Delta(k, v),
          CollectionDeltaWithPrevious::Remove(k, _) => CollectionDelta::Remove(k),
        })
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, K, V> VirtualCollection<K, V> for IntoReactiveCollection<T, K, V>
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
