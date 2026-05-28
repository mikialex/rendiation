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

  fn has_item_hint(&self) -> bool {
    self.base.has_item_hint()
  }
}

impl<T, U> DualQuery<T, U> {
  pub fn filter_map<K, V, V2, F>(
    self,
    f: F,
  ) -> DualQuery<FilterMapQuery<T, F>, FilterMapQueryChange<U, F>>
  where
    K: CKey,
    V: CValue,
    V2: CValue,
    T: Query<Key = K, Value = V>,
    U: Query<Key = K, Value = ValueChange<V>>,
    F: Fn(V) -> Option<V2> + Clone + Sync + Send + 'static,
  {
    DualQuery {
      view: self.view.filter_map(f.clone()),
      delta: self.delta.delta_filter_map(f),
    }
  }
}

#[test]
fn test_filter_map_query_change() {
  let mut base = FastHashMap::default();
  // both new and old pass the filter
  base.insert(1u32, ValueChange::Delta(10i32, Some(8)));
  // doesn't pass filter at all
  base.insert(2, ValueChange::Delta(3, None));
  // old value passes → Remove
  base.insert(3, ValueChange::Remove(8));

  let filtered = FilterMapQueryChange {
    base,
    mapper: |v: i32| if v > 5 { Some(v * 10) } else { None },
  };

  validate_query_consistency(&filtered);

  assert_eq!(
    filtered.access(&1),
    Some(ValueChange::Delta(100, Some(80)))
  );
  assert_eq!(filtered.access(&2), None);
  assert_eq!(filtered.access(&3), Some(ValueChange::Remove(80)));
}
