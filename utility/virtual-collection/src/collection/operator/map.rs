use crate::*;

#[derive(Clone)]
pub struct MappedCollection<'a, K, V, F> {
  pub base: Box<dyn VirtualCollection<K, V> + 'a>,
  pub mapper: F,
}

impl<'a, K, V, V2, F> VirtualCollection<K, V2> for MappedCollection<'a, K, V, F>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(&K, V) -> V2 + Clone + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V2)> + '_> {
    Box::new(self.base.iter_key_value().map(|(k, v)| {
      let v = (self.mapper)(&k, v);
      (k, v)
    }))
  }

  fn access(&self, key: &K) -> Option<V2> {
    self.base.access(key).map(|v| (self.mapper)(key, v))
  }
}

#[derive(Clone)]
pub struct KeyDualMapCollection<'a, K, V, F1, F2> {
  pub base: Box<dyn VirtualCollection<K, V> + 'a>,
  pub f1: F1,
  pub f2: F2,
}

impl<'a, K, K2, V, F1, F2> VirtualCollection<K2, V> for KeyDualMapCollection<'a, K, V, F1, F2>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K2, V)> + '_> {
    Box::new(self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    }))
  }

  fn access(&self, key: &K2) -> Option<V> {
    self.base.access(&(self.f2)(key.clone())?)
  }
}
