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
  type Changes = impl VirtualCollection<Self::Key, ValueChange<Self::Value>>;
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
  pub cache: Arc<RwLock<IndexKeptVec<Map::Value>>>,
}

impl<Map> ReactiveCollection for LinearMaterializedReactiveCollection<Map>
where
  Map: ReactiveCollection + Sync,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Changes = impl VirtualCollection<Self::Key, ValueChange<Self::Value>>;
  type View = LockReadGuardHolder<IndexKeptVec<Self::Value>>;
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
