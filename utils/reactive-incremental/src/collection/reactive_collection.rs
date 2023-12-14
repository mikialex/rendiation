use std::ops::DerefMut;

use crate::*;

#[derive(Clone, Copy)]
pub enum CPoll<T> {
  Ready(T),
  Blocked,
}

impl<T> CPoll<T> {
  pub fn is_blocked(&self) -> bool {
    matches!(self, CPoll::Blocked)
  }
  pub fn map<T2>(self, f: impl FnOnce(T) -> T2) -> CPoll<T2> {
    match self {
      CPoll::Ready(v) => CPoll::Ready(f(v)),
      CPoll::Blocked => CPoll::Blocked,
    }
  }

  pub fn unwrap(self) -> T {
    match self {
      CPoll::Ready(v) => v,
      CPoll::Blocked => panic!("failed to unwrap c poll"),
    }
  }
}

pub enum ExtraCollectionOperation {
  MemoryShrinkToFit,
}

pub type CollectionChanges<'a, K, V> = Box<dyn VirtualCollection<K, CollectionDelta<K, V>> + 'a>;
pub type PollCollectionChanges<'a, K, V> = CPoll<Poll<CollectionChanges<'a, K, V>>>;
pub type CollectionView<'a, K, V> = Box<dyn VirtualCollection<K, V> + 'a>;
pub type PollCollectionCurrent<'a, K, V> = CPoll<CollectionView<'a, K, V>>;

pub trait ReactiveCollection<K: Send, V: Send>: Sync + Send + 'static {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V>;

  fn access(&self) -> PollCollectionCurrent<K, V>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn spin_get_current(&self) -> Box<dyn VirtualCollection<K, V> + '_> {
    loop {
      match self.access() {
        CPoll::Ready(r) => return r,
        CPoll::Blocked => continue,
      }
    }
  }

  fn spin_poll_until_pending(
    &mut self,
    cx: &mut Context,
    consumer: &mut dyn FnMut(&dyn VirtualCollection<K, CollectionDelta<K, V>>),
  ) {
    loop {
      match self.poll_changes(cx) {
        CPoll::Ready(r) => {
          if let Poll::Ready(change) = r {
            consumer(change.as_ref())
          }
        }
        CPoll::Blocked => continue,
      }
    }
  }
}

#[derive(Clone)]
pub struct CollectionPreviousView<'a, K, V> {
  current: &'a dyn VirtualCollection<K, V>,
  delta: Option<&'a dyn VirtualCollection<K, CollectionDelta<K, V>>>,
}
pub fn make_previous<'a, K, V>(
  current: &'a dyn VirtualCollection<K, V>,
  delta: &'a Poll<Box<dyn VirtualCollection<K, CollectionDelta<K, V>> + 'a>>,
) -> CollectionPreviousView<'a, K, V> {
  let delta = match delta {
    Poll::Ready(v) => Some(v.as_ref()),
    Poll::Pending => None,
  };
  CollectionPreviousView { current, delta }
}

/// the impl access the previous V
impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for CollectionPreviousView<'a, K, V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    let current_not_changed = self.current.iter_key_value().filter(|(k, _)| {
      if let Some(delta) = &self.delta {
        !delta.contains(k)
      } else {
        true
      }
    });

    if let Some(delta) = &self.delta {
      let current_changed = delta
        .iter_key_value()
        .filter_map(|(k, v)| v.old_value().map(|v| (k, v.clone())));
      Box::new(current_not_changed.chain(current_changed))
    } else {
      Box::new(current_not_changed)
    }
  }

  fn access(&self, key: &K) -> Option<V> {
    if let Some(delta) = &self.delta {
      if let Some(change) = delta.access(key) {
        change.old_value().cloned()
      } else {
        self.current.access(key)
      }
    } else {
      self.current.access(key)
    }
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for () {
  fn poll_changes(&self, _: &mut Context<'_>) -> PollCollectionChanges<K, V> {
    CPoll::Ready(Poll::Pending)
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}

  fn access(&self) -> PollCollectionCurrent<K, V> {
    CPoll::Ready(Box::new(()))
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for Box<dyn ReactiveCollection<K, V>> {
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V> {
    self.deref().poll_changes(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request(request)
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.deref().access()
  }
}
