use crate::*;

#[derive(Clone)]
pub struct MappedCollection<K, V, F, T> {
  pub base: T,
  pub mapper: F,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, V2, F, T> VirtualCollection<K, V2> for MappedCollection<K, V, F, T>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(&K, V) -> V2 + Clone + Send + Sync + 'static,
  T: VirtualCollection<K, V>,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V2)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let v = (self.mapper)(&k, v);
      (k, v)
    })
  }

  fn access(&self, key: &K) -> Option<V2> {
    self.base.access(key).map(|v| (self.mapper)(key, v))
  }
}

#[derive(Clone)]
pub struct KeyDualMapCollection<K, V, F1, F2, T> {
  pub base: T,
  pub f1: F1,
  pub f2: F2,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, K2, V, F1, F2, T> VirtualCollection<K2, V> for KeyDualMapCollection<K, V, F1, F2, T>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  T: VirtualCollection<K, V>,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K2, V)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    })
  }

  fn access(&self, key: &K2) -> Option<V> {
    self.base.access(&(self.f2)(key.clone())?)
  }
}
