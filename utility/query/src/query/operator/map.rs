use crate::*;

#[derive(Clone)]
pub struct MappedQuery<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<V2, F, T> Query for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: Query,
{
  type Key = T::Key;
  type Value = V2;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, V2)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let v = (self.mapper)(&k, v);
      (k, v)
    })
  }

  fn access(&self, key: &T::Key) -> Option<V2> {
    self.base.access(key).map(|v| (self.mapper)(key, v))
  }
}

#[derive(Clone)]
pub struct KeyDualMappedQuery<T, F1, F2> {
  pub base: T,
  pub f1: F1,
  pub f2: F2,
}

impl<K2, F1, F2, T> Query for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<T::Key> + Clone + Send + Sync + 'static,
  T: Query,
{
  type Key = K2;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (K2, T::Value)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    })
  }

  fn access(&self, key: &K2) -> Option<T::Value> {
    self.base.access(&(self.f2)(key.clone())?)
  }
}
