use crate::*;

#[derive(Clone)]
pub struct CollectionFilter<K, V, F, T> {
  pub base: T,
  pub mapper: F,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, F, V2, T> VirtualCollection<K, V2> for CollectionFilter<K, V, F, T>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
  K: CKey,
  V: CValue,
  V2: CValue,
  T: VirtualCollection<K, V>,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V2)> + '_ {
    self
      .base
      .iter_key_value()
      .filter_map(|(k, v)| (self.mapper)(v).map(|v| (k, v)))
  }

  fn access(&self, key: &K) -> Option<V2> {
    let base = self.base.access(key)?;
    (self.mapper)(base)
  }
}
