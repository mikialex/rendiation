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
  type Changes = impl VirtualCollection<K, ValueChange<V2>>;
  type View = impl VirtualCollection<K, V2>;

  #[tracing::instrument(skip_all, name = "ReactiveKVMap")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);
    let map = self.map;
    let d = d.map(move |k, v| v.map(|v| map(k, v)));

    let v = v.map(self.map);

    (d, v)
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
  type Changes = impl VirtualCollection<K2, ValueChange<V>>;
  type View = impl VirtualCollection<K2, V>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);
    let d = d.key_dual_map(self.f1, self.f2);
    let v = v.key_dual_map(self.f1, self.f2);
    (d, v)
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
  type Changes = impl VirtualCollection<K, ValueChange<V2>>;
  type View = impl VirtualCollection<K, V2>;

  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.poll_changes(cx);

    let mapper = (self.map_creator)();
    let materialized = d.iter_key_value().collect::<Vec<_>>();
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
    let d = Arc::new(materialized);
    drop(cache);

    let v = self.cache.make_read_holder();
    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
    self.inner.extra_request(request)
  }
}
