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

  fn has_item_hint(&self) -> bool {
    self.current.has_item_hint() || self.delta.has_item_hint()
  }
}

#[test]
fn test_query_previous_view() {
  let mut current = FastHashMap::default();
  current.insert(1u32, "a".to_string());
  current.insert(2, "b".to_string());
  current.insert(3, "c".to_string());

  // delta: key 1 changed from "a" to "a2", key 2 removed
  let mut delta = FastHashMap::default();
  delta.insert(
    1u32,
    ValueChange::Delta("a2".to_string(), Some("a".to_string())),
  );
  delta.insert(2, ValueChange::Remove("b".to_string()));

  let prev = make_previous(&current, &delta);

  validate_query_consistency(&prev);

  // key 1: previous value from delta
  assert_eq!(prev.access(&1), Some("a".to_string()));
  // key 2: removed, previous value from delta
  assert_eq!(prev.access(&2), Some("b".to_string()));
  // key 3: unchanged, from current
  assert_eq!(prev.access(&3), Some("c".to_string()));
  // key 4: doesn't exist
  assert_eq!(prev.access(&4), None);
}

#[test]
fn test_query_previous_view_no_delta() {
  let mut current = FastHashMap::default();
  current.insert(1u32, "a".to_string());

  let delta: FastHashMap<u32, ValueChange<String>> = FastHashMap::default();
  let prev = make_previous(&current, &delta);

  validate_query_consistency(&prev);
  assert_eq!(prev.access(&1), Some("a".to_string()));
}

#[test]
fn test_query_previous_view_insert_only() {
  let current: FastHashMap<u32, String> = FastHashMap::default();

  let mut delta = FastHashMap::default();
  delta.insert(1u32, ValueChange::Delta("new".to_string(), None));

  let prev = make_previous(&current, &delta);

  // newly inserted key has no previous value
  assert_eq!(prev.access(&1), None);
}
