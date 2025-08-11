use crate::*;

#[derive(Clone)]
pub struct FilterMapQueryChange<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<F, V, V2, T> Query for FilterMapQueryChange<T, F>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
  V2: CValue,
  T: Query<Value = ValueChange<V>>,
{
  type Key = T::Key;
  type Value = ValueChange<V2>;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, ValueChange<V2>)> + '_ {
    let checker = make_checker(self.mapper.clone());
    self
      .base
      .iter_key_value()
      .filter_map(move |(k, v)| (checker)(v).map(|v| (k, v)))
  }

  fn access(&self, key: &T::Key) -> Option<ValueChange<V2>> {
    let checker = make_checker(self.mapper.clone());
    let base = self.base.access(key)?;
    (checker)(base)
  }
}

pub trait DeltaQueryExt: Query {
  fn delta_filter_map<V, V2, F>(self, mapper: F) -> FilterMapQueryChange<Self, F>
  where
    F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
    Self: Query<Value = ValueChange<V>>,
    V2: CValue,
  {
    FilterMapQueryChange { base: self, mapper }
  }
}
impl<T: Query> DeltaQueryExt for T {}
