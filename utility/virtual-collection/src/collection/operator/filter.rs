use crate::*;

#[derive(Clone)]
pub struct CollectionFilter<F, T> {
  pub base: T,
  pub mapper: F,
}

impl<F, V2, T> VirtualCollection for CollectionFilter<F, T>
where
  F: Fn(T::Value) -> Option<V2> + Sync + Send + Clone + 'static,
  V2: CValue,
  T: VirtualCollection,
{
  type Key = T::Key;
  type Value = V2;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, V2)> + '_ {
    self
      .base
      .iter_key_value()
      .filter_map(|(k, v)| (self.mapper)(v).map(|v| (k, v)))
  }

  fn access(&self, key: &T::Key) -> Option<V2> {
    let base = self.base.access(key)?;
    (self.mapper)(base)
  }
}
