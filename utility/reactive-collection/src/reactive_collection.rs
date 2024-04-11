use std::ops::DerefMut;

use crate::*;

pub enum ExtraCollectionOperation {
  MemoryShrinkToFit,
}

pub type CollectionChanges<K, V> = Box<dyn VirtualCollection<K, ValueChange<V>>>;
pub type PollCollectionChanges<K, V> = Poll<CollectionChanges<K, V>>;
pub type CollectionView<K, V> = Box<dyn VirtualCollection<K, V>>;
pub type PollCollectionCurrent<K, V> = CollectionView<K, V>;

pub trait ReactiveCollection<K: CKey, V: CValue>: Sync + Send + 'static {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V>;

  fn access(&self) -> PollCollectionCurrent<K, V>;

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);

  fn spin_poll_until_pending(
    &mut self,
    cx: &mut Context,
    consumer: &mut dyn FnMut(&dyn VirtualCollection<K, ValueChange<V>>),
  ) {
    loop {
      let r = self.poll_changes(cx);
      if let Poll::Ready(change) = r {
        consumer(change.as_ref())
      } else {
        return;
      }
    }
  }
}

#[derive(Clone)]
pub struct CollectionPreviousView<'a, K, V> {
  current: &'a dyn VirtualCollection<K, V>,
  delta: Option<&'a dyn VirtualCollection<K, ValueChange<V>>>,
}
pub fn make_previous<'a, K, V>(
  current: &'a dyn VirtualCollection<K, V>,
  delta: &'a Poll<Box<dyn VirtualCollection<K, ValueChange<V>> + 'a>>,
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
  fn poll_changes(&self, _: &mut Context) -> PollCollectionChanges<K, V> {
    Poll::Pending
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}

  fn access(&self) -> PollCollectionCurrent<K, V> {
    Box::new(())
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for Box<dyn ReactiveCollection<K, V>> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self.deref().poll_changes(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request(request)
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.deref().access()
  }
}
