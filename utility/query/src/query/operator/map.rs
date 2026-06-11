use crate::*;

#[derive(Clone)]
pub struct MappedQuery<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<V2, F, T> Query for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: Query,
{
  type Key = T::Key;
  type Value = V2;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, V2)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let v = (self.mapper)(&k, v);
      (k, v)
    })
  }

  fn access(&self, key: &T::Key) -> Option<V2> {
    self.base.access(key).map(|v| (self.mapper)(key, v))
  }

  fn has_item_hint(&self) -> bool {
    self.base.has_item_hint()
  }
}

#[test]
fn test_mapped_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, 10);
  base.insert(2, 20);
  base.insert(3, 30);

  let mapped = MappedQuery {
    base,
    mapper: |k: &u32, v: i32| format!("{}:{}", k, v),
  };

  super::validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&1), Some("1:10".to_string()));
  assert_eq!(mapped.access(&2), Some("2:20".to_string()));
  assert_eq!(mapped.access(&3), Some("3:30".to_string()));
  assert_eq!(mapped.access(&4), None);
}

#[derive(Clone)]
pub struct MappedValueQuery<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<V2, F, T> Query for MappedValueQuery<T, F>
where
  V2: CValue,
  F: Fn(T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: Query,
{
  type Key = T::Key;
  type Value = V2;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, V2)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let v = (self.mapper)(v);
      (k, v)
    })
  }

  fn access(&self, key: &T::Key) -> Option<V2> {
    self.base.access(key).map(|v| (self.mapper)(v))
  }

  fn has_item_hint(&self) -> bool {
    self.base.has_item_hint()
  }
}

#[test]
fn test_mapped_value_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, 10);
  base.insert(2, 20);

  let mapped = MappedValueQuery {
    base,
    mapper: |v: i32| v * 2,
  };

  super::validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&1), Some(20));
  assert_eq!(mapped.access(&2), Some(40));
  assert_eq!(mapped.access(&3), None);
}

#[test]
fn test_mapped_empty_query() {
  let base: FastHashMap<u32, i32> = FastHashMap::default();
  let mapped = MappedValueQuery {
    base,
    mapper: |v: i32| v * 2,
  };

  super::validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&1), None);
}

#[test]
fn test_mapped_value_multi_query() {
  let mut base: FastHashMap<u32, FastHashSet<i32>> = FastHashMap::default();
  base.insert(1, FastHashSet::from_iter([10, 20]));
  base.insert(2, FastHashSet::from_iter([30]));

  let mapped = MappedValueQuery {
    base,
    mapper: |v: i32| v * 2,
  };

  validate_multi_query_consistency(&mapped);

  let values: Vec<_> = mapped.access_multi(&1).unwrap().collect();
  assert_eq!(values.len(), 2);
  assert!(values.contains(&20));
  assert!(values.contains(&40));

  let values: Vec<_> = mapped.access_multi(&2).unwrap().collect();
  assert_eq!(values.len(), 1);
  assert!(values.contains(&60));

  assert!(mapped.access_multi(&3).is_none());
}

#[derive(Clone)]
pub struct KeyDualMappedQuery<T, F1, F2> {
  pub base: T,
  pub f1: F1,
  pub f2: F2,
}

impl<K2, F1, F2, T> Query for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<T::Key> + Clone + Send + Sync + 'static,
  T: Query,
{
  type Key = K2;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (K2, T::Value)> + '_ {
    self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    })
  }

  fn access(&self, key: &K2) -> Option<T::Value> {
    self.base.access(&(self.f2)(key.clone())?)
  }

  fn has_item_hint(&self) -> bool {
    self.base.has_item_hint()
  }
}

#[test]
fn test_key_dual_mapped_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, "a".to_string());
  base.insert(2, "b".to_string());

  let mapped = KeyDualMappedQuery {
    base,
    f1: |k: u32| k + 100,
    f2: |k: u32| if k > 100 { Some(k - 100) } else { None },
  };

  super::validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&101), Some("a".to_string()));
  assert_eq!(mapped.access(&102), Some("b".to_string()));
  assert_eq!(mapped.access(&1), None);
  assert_eq!(mapped.access(&103), None);
}

#[test]
fn test_key_dual_mapped_multi_query() {
  let mut base: FastHashMap<u32, FastHashSet<String>> = FastHashMap::default();
  base.insert(1, FastHashSet::from_iter(["a".to_string()]));
  base.insert(
    2,
    FastHashSet::from_iter(["b".to_string(), "c".to_string()]),
  );

  let mapped = KeyDualMappedQuery {
    base,
    f1: |k: u32| k + 100,
    f2: |k: u32| if k > 100 { Some(k - 100) } else { None },
  };

  validate_multi_query_consistency(&mapped);

  let values: Vec<_> = mapped.access_multi(&101).unwrap().collect();
  assert_eq!(values.len(), 1);
  assert!(values.contains(&"a".to_string()));

  let values: Vec<_> = mapped.access_multi(&102).unwrap().collect();
  assert_eq!(values.len(), 2);
  assert!(values.contains(&"b".to_string()));
  assert!(values.contains(&"c".to_string()));

  assert!(mapped.access_multi(&1).is_none());
  assert!(mapped.access_multi(&103).is_none());
}
