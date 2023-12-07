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

pub type CollectionChangesWithPrevious<K, V> = FastHashMap<K, CollectionDeltaWithPrevious<K, V>>;

pub trait ReactiveCollectionWithPrevious<K: Send, V: Send>:
  VirtualCollection<K, V> + Sync + Send + 'static
{
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<CollectionChangesWithPrevious<K, V>>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn spin_poll_until_pending(
    &mut self,
    cx: &mut Context,
    mut consumer: impl FnMut(CollectionChangesWithPrevious<K, V>),
  ) {
    loop {
      match self.poll_changes(cx) {
        CPoll::Ready(r) => consumer(r),
        CPoll::Pending => return,
        CPoll::Blocked => continue,
      }
    }
  }

  fn into_forker(self) -> ReactiveKVMapFork<Self, CollectionChangesWithPrevious<K, V>, K, V>
  where
    Self: Sized,
  {
    BufferedCollection::new(ReactiveKVMapForkImpl::new(self))
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
    K: Send + Sync + 'static + Clone + Eq + std::hash::Hash,
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
  ) -> CPoll<CollectionChangesWithPrevious<K, V>>;
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
  ) -> CPoll<CollectionChangesWithPrevious<K, V>> {
    self.poll_changes(cx)
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
  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.deref().try_access_boxed()
  }
}
impl<K, V> ReactiveCollectionWithPrevious<K, V>
  for Box<dyn DynamicReactiveCollectionWithPrevious<K, V>>
where
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChangesWithPrevious<K, V>> {
    self.deref_mut().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}

impl<K, V> CollectionDeltaWithPrevious<K, V> {
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k, _) => k,
      Self::Delta(k, _, _) => k,
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
}

struct IntoReactiveCollection<T, K, V> {
  inner: T,
  phantom: PhantomData<(K, V)>,
}

impl<T, K, V> ReactiveCollection<K, V> for IntoReactiveCollection<T, K, V>
where
  T: ReactiveCollectionWithPrevious<K, V>,
  K: Send + Sync + 'static + Clone + Eq + std::hash::Hash,
  V: Send + Sync + 'static + Clone,
{
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<CollectionChanges<K, V>> {
    self.inner.poll_changes(cx).map(|v| {
      v.into_iter()
        .map(|(k, v)| {
          (
            k,
            match v {
              CollectionDeltaWithPrevious::Delta(k, v, _) => CollectionDelta::Delta(k, v),
              CollectionDeltaWithPrevious::Remove(k, _) => CollectionDelta::Remove(k),
            },
          )
        })
        .collect()
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

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    self.inner.try_access()
  }
}
