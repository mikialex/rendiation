use crate::*;

#[derive(Clone)]
pub struct CollectionFilter<'a, K, V, F> {
  pub base: Box<dyn VirtualCollection<K, V> + 'a>,
  pub mapper: F,
}

impl<'a, K, V, F, V2> VirtualCollection<K, V2> for CollectionFilter<'a, K, V, F>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
  K: CKey,
  V: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V2)> + '_> {
    Box::new(
      self
        .base
        .iter_key_value()
        .filter_map(|(k, v)| (self.mapper)(v).map(|v| (k, v))),
    )
  }

  fn access(&self, key: &K) -> Option<V2> {
    let base = self.base.access(key)?;
    (self.mapper)(base)
  }
}
