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
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = LockReadGuardHolder<FastHashMap<K, V>>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.poll_changes(cx);
    {
      let mut cache = self.cache.write();
      for (k, change) in d.iter_key_value() {
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

    let v = self.cache.make_read_holder();

    (d, v)
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
}

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  K: LinearIdentification + CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = LockReadGuardHolder<IndexKeptVec<V>>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.poll_changes(cx);
    {
      let mut cache = self.cache.write();
      for (k, change) in d.iter_key_value() {
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

    let v = self.cache.make_read_holder();

    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }
}
