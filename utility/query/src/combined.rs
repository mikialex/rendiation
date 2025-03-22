use crate::*;

#[derive(Clone)]
pub struct QueryAndMultiQuery<T, M> {
  pub query: T,
  pub multi_query: M,
}

impl<T, M> Query for QueryAndMultiQuery<T, M>
where
  T: Query,
  M: MultiQuery,
{
  type Key = T::Key;
  type Value = T::Value;
  fn access(&self, m: &T::Key) -> Option<T::Value> {
    self.query.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.query.iter_key_value()
  }
}

impl<T, M> MultiQuery for QueryAndMultiQuery<T, M>
where
  T: Query,
  M: MultiQuery,
{
  type Key = M::Key;
  type Value = M::Value;
  fn iter_keys(&self) -> impl Iterator<Item = M::Key> + '_ {
    self.multi_query.iter_keys()
  }

  fn access_multi(&self, key: &M::Key) -> Option<impl Iterator<Item = M::Value> + '_> {
    self.multi_query.access_multi(key)
  }
}
