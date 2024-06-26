use crate::*;

pub struct UnorderedMaterializedReactiveCollection<Map, K, V> {
  pub inner: Map,
  pub cache: Arc<RwLock<FastHashMap<K, V>>>,
}

impl<Map, K, V> ReactiveCollection<K, V> for UnorderedMaterializedReactiveCollection<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = LockReadGuardHolder<FastHashMap<K, V>>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f = self.inner.poll_changes(cx);
    let cache = self.cache.clone();
    async move {
      let (d, _) = f.await;
      {
        let mut cache = cache.write();
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

      let v = cache.make_read_holder();
      (d, v)
    }
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

impl<Map, K, V> ReactiveCollection<K, V> for LinearMaterializedReactiveCollection<Map, V>
where
  Map: ReactiveCollection<K, V> + Sync,
  K: LinearIdentification + CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = LockReadGuardHolder<IndexKeptVec<V>>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f = self.inner.poll_changes(cx);
    let cache = self.cache.clone();
    async move {
      let (d, _) = f.await;
      {
        let mut cache = cache.write();
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

      let v = cache.make_read_holder();

      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }
}
