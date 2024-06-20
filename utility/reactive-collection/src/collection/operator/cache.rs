use crate::*;

pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  pub inner: Map,
  pub cache: Arc<RwLock<FastHashMap<K, V>>>,
}

impl<Map, K, V> ReactiveCollectionSelfContained<K, V>
  for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V>> {
    Box::new(self.cache.make_read_holder())
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self.inner.poll_changes(cx).map(|changes| {
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
      changes
    })
  }
  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.cache.make_lock_holder_collection()
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
  pub cache: Arc<RwLock<IndexKeptVec<V>>>,
}

impl<Map, K, V> ReactiveCollectionSelfContained<K, V>
  for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V>,
  K: LinearIdentification + CKey,
  V: CValue,
{
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V>> {
    Box::new(self.cache.make_read_holder())
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  K: LinearIdentification + CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self.inner.poll_changes(cx).map(|changes| {
      let mut cache = self.cache.write();
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
      changes
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.cache.make_lock_holder_collection()
  }
}
