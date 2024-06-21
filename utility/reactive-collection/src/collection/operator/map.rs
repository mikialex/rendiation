use crate::*;

pub struct ReactiveKVMap<T, F, K, V> {
  pub inner: T,
  pub map: F,
  pub phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  V: CValue,
  K: CKey,
  V2: CValue,
  F: Fn(&K, V) -> V2 + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V>,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVMap")]
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V2> {
    let map = self.map;
    self
      .inner
      .poll_changes(cx)
      .map(move |delta| delta.map(move |k, v| v.map(|v| map(k, v))).into_boxed())
  }

  fn access(&self) -> PollCollectionCurrent<K, V2> {
    self.inner.access().map(self.map).into_boxed()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

pub struct ReactiveKeyDualMap<F1, F2, T, K, V> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<F1, F2, T, K, K2, V> ReactiveCollection<K2, V> for ReactiveKeyDualMap<F1, F2, T, K, V>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> K + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K2, V> {
    self
      .inner
      .poll_changes(cx)
      .map(|delta| delta.key_dual_map(self.f1, self.f2).into_boxed())
  }

  fn access(&self) -> PollCollectionCurrent<K2, V> {
    self
      .inner
      .access()
      .key_dual_map(self.f1, self.f2)
      .into_boxed()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T, F, K, V, V2> {
  pub inner: T,
  pub map_creator: F,
  pub cache: Arc<RwLock<FastHashMap<K, V2>>>,
  pub phantom: PhantomData<(K, V, V2)>,
}

impl<T, F, K, V, V2, FF> ReactiveCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V, V2>
where
  V: CValue,
  K: CKey,
  F: Fn() -> FF + Send + Sync + 'static,
  FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: ReactiveCollection<K, V>,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V2> {
    self.inner.poll_changes(cx).map(move |deltas| {
      let mapper = (self.map_creator)();
      let materialized = deltas.iter_key_value().collect::<Vec<_>>();
      let mut cache = self.cache.write();
      let materialized: FastHashMap<K, ValueChange<V2>> = materialized
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

      Box::new(Arc::new(materialized)) as Box<dyn DynVirtualCollection<K, ValueChange<V2>>>
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
    self.inner.extra_request(request)
  }

  fn access(&self) -> PollCollectionCurrent<K, V2> {
    Box::new(self.cache.make_read_holder())
  }
}
