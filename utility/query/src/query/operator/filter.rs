use crate::*;

#[derive(Clone)]
pub struct FilterMapQuery<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<F, V2, T> Query for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Sync + Send + Clone + 'static,
  V2: CValue,
  T: Query,
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

  fn has_item_hint(&self) -> bool {
    self.base.has_item_hint()
  }
}

#[test]
fn test_filter_map_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, 10);
  base.insert(2, 15);
  base.insert(3, 20);
  base.insert(4, 5);

  let filtered = FilterMapQuery {
    base,
    mapper: |v: i32| if v >= 15 { Some(v * 2) } else { None },
  };

  super::validate_query_consistency(&filtered);
  assert_eq!(filtered.access(&1), None);
  assert_eq!(filtered.access(&2), Some(30));
  assert_eq!(filtered.access(&3), Some(40));
  assert_eq!(filtered.access(&4), None);
  assert_eq!(filtered.access(&5), None);
}

#[test]
fn test_filter_map_all_passed() {
  let mut base = FastHashMap::default();
  base.insert(1u32, "hello".to_string());
  base.insert(2, "world".to_string());

  let filtered = FilterMapQuery {
    base,
    mapper: |v: String| Some(v),
  };

  super::validate_query_consistency(&filtered);
  assert_eq!(filtered.access(&1), Some("hello".to_string()));
  assert_eq!(filtered.access(&2), Some("world".to_string()));
}

#[test]
fn test_filter_map_all_filtered() {
  let mut base = FastHashMap::default();
  base.insert(1u32, 10);
  base.insert(2, 20);

  let filtered = FilterMapQuery {
    base,
    mapper: |_: i32| -> Option<i32> { None },
  };

  super::validate_query_consistency(&filtered);
  assert_eq!(filtered.access(&1), None);
  assert_eq!(filtered.access(&2), None);
}
