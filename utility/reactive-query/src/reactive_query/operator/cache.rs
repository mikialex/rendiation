use crate::*;

pub struct UnorderedMaterializedViewCache<T, K: CKey, V: CValue> {
  pub inner: T,
  pub cache: Arc<RwLock<FastHashMap<K, V>>>,
}

impl<T: QueryCompute> QueryCompute for UnorderedMaterializedViewCache<T, T::Key, T::Value> {
  type Key = T::Key;
  type Value = T::Value;
  type Changes = T::Changes;
  type View = LockReadGuardHolder<FastHashMap<T::Key, T::Value>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.resolve(cx);
    cx.keep_view_alive(v);
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
    drop(cache);

    let v = self.cache.make_read_holder();
    (d, v)
  }
}

impl<T: AsyncQueryCompute> AsyncQueryCompute
  for UnorderedMaterializedViewCache<T, T::Key, T::Value>
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let cache = self.cache.clone();
    let inner = self.inner.create_task(cx);
    cx.then_spawn_compute(inner, |inner| UnorderedMaterializedViewCache {
      inner,
      cache,
    })
    .into_boxed_future()
  }
}

impl<Map> ReactiveQuery for UnorderedMaterializedViewCache<Map, Map::Key, Map::Value>
where
  Map: ReactiveQuery,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = UnorderedMaterializedViewCache<Map::Compute, Self::Key, Self::Value>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    UnorderedMaterializedViewCache {
      inner: self.inner.describe(cx),
      cache: self.cache.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
  }
}

pub struct LinearMaterializedReactiveQuery<Map, K, V> {
  pub inner: Map,
  pub cache: Arc<RwLock<IndexReusedVecAccess<K, V>>>,
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

impl<Map> QueryCompute for LinearMaterializedReactiveQuery<Map, Map::Key, Map::Value>
where
  Map: QueryCompute,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  type Key = Map::Key;
  type Value = Map::Value;

  type Changes = Map::Changes;
  type View = LockReadGuardHolder<IndexReusedVecAccess<Map::Key, Map::Value>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.resolve(cx);
    cx.keep_view_alive(v);
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
}

impl<Map> AsyncQueryCompute for LinearMaterializedReactiveQuery<Map, Map::Key, Map::Value>
where
  Map: AsyncQueryCompute,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let cache = self.cache.clone();
    let inner = self.inner.create_task(cx);
    cx.then_spawn_compute(inner, |inner| LinearMaterializedReactiveQuery {
      inner,
      cache,
    })
    .into_boxed_future()
  }
}

impl<Map> ReactiveQuery for LinearMaterializedReactiveQuery<Map, Map::Key, Map::Value>
where
  Map: ReactiveQuery + Sync,
  Map::Key: LinearIdentification + CKey,
  Map::Value: CValue,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = LinearMaterializedReactiveQuery<Map::Compute, Map::Key, Map::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    LinearMaterializedReactiveQuery {
      inner: self.inner.describe(cx),
      cache: self.cache.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().inner.shrink_to_fit(),
    }
  }
}
