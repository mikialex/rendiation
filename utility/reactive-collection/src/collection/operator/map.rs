use crate::*;

pub struct ReactiveKVMap<T, F> {
  pub inner: T,
  pub map: F,
}

impl<T, F, V2> ReactiveCollection for ReactiveKVMap<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Copy + Send + Sync + 'static,
  T: ReactiveCollection,
{
  type Key = T::Key;
  type Value = V2;
  type Changes = impl VirtualCollection<Key = Self::Key, Value = ValueChange<V2>>;
  type View = impl VirtualCollection<Key = Self::Key, Value = V2>;

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

pub struct ReactiveKeyDualMap<F1, F2, T> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
}

impl<F1, F2, T, K2> ReactiveCollection for ReactiveKeyDualMap<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: ReactiveCollection,
{
  type Key = K2;
  type Value = T::Value;
  type Changes = impl VirtualCollection<Key = K2, Value = ValueChange<T::Value>>;
  type View = impl VirtualCollection<Key = K2, Value = T::Value>;

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
pub struct ReactiveKVExecuteMap<T: ReactiveCollection, F, V2> {
  pub inner: T,
  pub map_creator: F,
  pub cache: Arc<RwLock<FastHashMap<T::Key, V2>>>,
}

impl<T, F, V2, FF> ReactiveCollection for ReactiveKVExecuteMap<T, F, V2>
where
  F: Fn() -> FF + Send + Sync + 'static,
  FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: ReactiveCollection,
{
  type Key = T::Key;
  type Value = V2;
  type Changes = impl VirtualCollection<Key = Self::Key, Value = ValueChange<V2>>;
  type View = impl VirtualCollection<Key = Self::Key, Value = V2>;

  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.inner.poll_changes(cx);

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

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
    self.inner.extra_request(request)
  }
}
