use crate::*;

#[derive(Clone)]
pub struct QueryPreviousView<C, D> {
  current: C,
  delta: D,
}
pub fn make_previous<C, D>(current: C, delta: D) -> QueryPreviousView<C, D> {
  QueryPreviousView { current, delta }
}

/// the impl access the previous V
impl<C, D, K, V> Query for QueryPreviousView<C, D>
where
  C: Query<Key = K, Value = V>,
  D: Query<Key = K, Value = ValueChange<V>>,
  K: CKey,
  V: CValue,
{
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    let current_not_changed = self
      .current
      .iter_key_value()
      .filter(|(k, _)| !self.delta.contains(k));

    let current_changed = self
      .delta
      .iter_key_value()
      .filter_map(|(k, v)| v.old_value().map(|v| (k, v.clone())));
    current_not_changed.chain(current_changed)
  }

  fn access(&self, key: &K) -> Option<V> {
    if let Some(change) = self.delta.access(key) {
      change.old_value().cloned()
    } else {
      self.current.access_dyn(key)
    }
  }
}
