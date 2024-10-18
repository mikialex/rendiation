use crate::*;

pub struct UnorderedMaterializedReactiveCollection<Map: ReactiveCollection> {
  pub inner: Map,
  pub cache: Arc<RwLock<FastHashMap<Map::Key, Map::Value>>>,
}

impl<Map> ReactiveCollection for UnorderedMaterializedReactiveCollection<Map>
where
  Map: ReactiveCollection,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Changes = impl VirtualCollection<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = LockReadGuardHolder<FastHashMap<Self::Key, Self::Value>>;
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

pub struct LinearMaterializedReactiveCollection<Map: ReactiveCollection> {
  pub inner: Map,
  pub cache: Arc<RwLock<IndexReusedVecAccess<Map::Key, Map::Value>>>,
}

#[derive(Clone)]
pub struct IndexReusedVecAccess<K, V> {
  inner: IndexKeptVec<V>,
  k: PhantomData<K>,
}

impl<K, V> Default for IndexReusedVecAccess<K, V> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      k: Default::default(),
    }
  }
}

impl<K: CKey + LinearIdentification, V: CValue> VirtualCollection for IndexReusedVecAccess<K, V> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .inner
      .iter_key_value()
      .map(|(k, v)| (K::from_alloc_index(k), v))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.inner.access(&key.alloc_index())
  }
}

impl<Map> ReactiveCollection for LinearMaterializedReactiveCollection<Map>
where
  Map: ReactiveCollection + Sync,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Changes = impl VirtualCollection<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = LockReadGuardHolder<IndexReusedVecAccess<Self::Key, Self::Value>>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.poll_changes(cx);
    {
      let mut cache = self.cache.write();
      for (k, change) in d.iter_key_value() {
        match change {
          ValueChange::Delta(v, _) => {
            cache.inner.insert(v, k.alloc_index());
          }
          ValueChange::Remove(_) => {
            cache.inner.remove(k.alloc_index());
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
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().inner.shrink_to_fit(),
    }
  }
}
