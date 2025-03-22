use crate::*;

pub struct UnorderedMaterializedReactiveQuery<Map: ReactiveQuery> {
  pub inner: Map,
  pub cache: Arc<RwLock<FastHashMap<Map::Key, Map::Value>>>,
}

pub struct UnorderedMaterializeCompute<T, K: CKey, V: CValue> {
  pub inner: T,
  pub cache: Option<LockWriteGuardHolder<FastHashMap<K, V>>>,
}

impl<T: QueryCompute> QueryCompute for UnorderedMaterializeCompute<T, T::Key, T::Value> {
  type Key = T::Key;
  type Value = T::Value;
  type Changes = impl Query<Key = T::Key, Value = ValueChange<T::Value>> + 'static;
  type View = impl Query<Key = T::Key, Value = T::Value> + 'static;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.resolve();
    let mut cache = self.cache.take().unwrap();

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

    let v = cache.downgrade_to_read();
    (d, v)
  }
}

// impl<T: AsyncQueryCompute> AsyncQueryCompute for UnorderedMaterializeCompute<T, T::Key, T::Value> {
//   type Task = impl Future<Output = (Self::Changes, Self::View)>;

//   fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
//     self.inner.create_task(cx).then(|inner| {
//       cx.spawn_task(|| {
//         UnorderedMaterializeCompute {
//           inner,
//           cache: self.cache.take(),
//         }
//         .resolve()
//       })
//     })
//   }
// }

impl<Map> ReactiveQuery for UnorderedMaterializedReactiveQuery<Map>
where
  Map: ReactiveQuery,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = UnorderedMaterializeCompute<Map::Compute, Self::Key, Self::Value>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    UnorderedMaterializeCompute {
      inner: self.inner.describe(cx),
      cache: Some(self.cache.make_write_holder()),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }
}

pub struct LinearMaterializedReactiveQuery<Map: ReactiveQuery> {
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

impl<K: CKey + LinearIdentification, V: CValue> Query for IndexReusedVecAccess<K, V> {
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

impl<Map> ReactiveQuery for LinearMaterializedReactiveQuery<Map>
where
  Map: ReactiveQuery + Sync,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = impl QueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let (d, _) = self.inner.describe(cx).resolve();
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

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().inner.shrink_to_fit(),
    }
  }
}
