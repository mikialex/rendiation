use crate::*;

impl<T, F, V2> ReactiveQuery for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: ReactiveQuery,
{
  type Key = T::Key;
  type Value = V2;
  type Compute = MappedQuery<T::Compute, F>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    MappedQuery {
      base: self.base.describe(cx),
      mapper: self.mapper.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<T, F, V2> QueryCompute for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: QueryCompute,
{
  type Key = T::Key;
  type Value = V2;

  type Changes = impl Query<Key = Self::Key, Value = ValueChange<V2>> + 'static;
  type View = MappedQuery<T::View, F>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve();
    let mapper = self.mapper.clone();
    let d = d.map(move |k, v| v.map(|v| mapper(k, v)));
    let v = v.map(self.mapper.clone());

    (d, v)
  }
}

impl<F1, F2, T, K2> ReactiveQuery for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: ReactiveQuery,
{
  type Key = K2;
  type Value = T::Value;
  type Compute = impl QueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    KeyDualMappedQuery {
      f1: self.f1,
      f2: self.f2,
      base: self.base.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<F1, F2, T, K2> QueryCompute for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: QueryCompute,
{
  type Key = K2;
  type Value = T::Value;

  type Changes = KeyDualMappedQuery<T::Changes, F1, AutoSomeFnResult<F2>>;
  type View = KeyDualMappedQuery<T::View, F1, AutoSomeFnResult<F2>>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve();
    let d = d.key_dual_map(self.f1, self.f2);
    let v = v.key_dual_map(self.f1, self.f2);
    (d, v)
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T: ReactiveQuery, F, V2> {
  pub inner: T,
  pub map_creator: F,
  pub cache: Arc<RwLock<FastHashMap<T::Key, V2>>>,
}

impl<T, F, V2, FF> ReactiveQuery for ReactiveKVExecuteMap<T, F, V2>
where
  F: Fn() -> FF + Send + Sync + 'static,
  FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: ReactiveQuery,
{
  type Key = T::Key;
  type Value = V2;
  type Compute = impl QueryCompute<Key = Self::Key, Value = Self::Value>;

  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let (d, _) = self.inner.describe(cx).resolve();

    let mut mapper = (self.map_creator)();
    let materialized = d.iter_key_value().collect::<Vec<_>>();
    let mut cache = self.cache.write();
    let materialized: FastHashMap<T::Key, ValueChange<V2>> = materialized
      .into_iter()
      .map(|(k, delta)| match delta {
        ValueChange::Delta(d, _p) => {
          let new_value = mapper(&k, d);
          let p = cache.insert(k.clone(), new_value.clone());
          (k, ValueChange::Delta(new_value, p))
        }
        ValueChange::Remove(_p) => {
          let p = cache.remove(&k).unwrap();
          (k, ValueChange::Remove(p))
        }
      })
      .collect();
    let d = Arc::new(materialized);
    drop(cache);

    let v = self.cache.make_read_holder();
    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
    self.inner.request(request)
  }
}
