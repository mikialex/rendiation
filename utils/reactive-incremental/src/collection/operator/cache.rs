use storage::IndexKeptVec;

use crate::*;

pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  pub inner: Map,
  pub cache: RwLock<FastHashMap<K, V>>,
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self.inner.poll_changes(cx).map(|delta| {
      if let Poll::Ready(changes) = &delta {
        let mut cache = self.cache.write();
        for (k, change) in changes.iter_key_value() {
          match change.clone() {
            ValueChange::Delta(v, _) => {
              cache.insert(k, v);
            }
            ValueChange::Remove(_) => {
              cache.remove(&k);
            }
          }
        }
      }
      delta
    })
  }
  fn access(&self) -> PollCollectionCurrent<K, V> {
    CPoll::Ready(self.cache.make_lock_holder_collection())
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }
}

pub struct LinearMaterializedReactiveCollection<Map, V> {
  pub inner: Map,
  pub cache: RwLock<IndexKeptVec<V>>,
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  K: LinearIdentification + CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self.inner.poll_changes(cx).map(|delta| {
      let mut cache = self.cache.write();
      if let Poll::Ready(changes) = &delta {
        for (k, change) in changes.iter_key_value() {
          match change {
            ValueChange::Delta(v, _) => {
              cache.insert(v, k.alloc_index());
            }
            ValueChange::Remove(_) => {
              cache.remove(k.alloc_index());
            }
          }
        }
      }
      delta
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    CPoll::Ready(self.cache.make_lock_holder_collection())
  }
}

impl<K: CKey + LinearIdentification, V: CValue> VirtualCollection<K, V> for IndexKeptVec<V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new(
      self
        .iter()
        .map(|(k, v)| (K::from_alloc_index(k), v.clone())),
    )
  }

  fn access(&self, key: &K) -> Option<V> {
    self.try_get(key.alloc_index()).cloned()
  }
}
