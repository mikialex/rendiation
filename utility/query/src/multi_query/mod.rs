use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

mod bookkeeping;
pub use bookkeeping::*;

pub trait MultiQuery: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_;
  /// if k is not in the query at all, return None.
  /// if k is in the query but map to none of v, return empty iterator
  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_>;
  fn access_multi_value(&self, key: &Self::Key) -> impl Iterator<Item = Self::Value> + '_ {
    self
      .access_multi(key)
      .map(|v| Box::new(v) as Box<dyn Iterator<Item = Self::Value> + '_>) // todo impl iterator for better performance
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor(&self, key: &Self::Key, visitor: &mut dyn FnMut(Self::Value)) {
    if let Some(v) = self.access_multi(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

impl<T: MultiQuery> MultiQuery for &T {
  type Key = T::Key;
  type Value = T::Value;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    (**self).iter_keys()
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    (**self).access_multi(key)
  }

  fn access_multi_value(&self, key: &Self::Key) -> impl Iterator<Item = Self::Value> + '_ {
    (**self).access_multi_value(key)
  }

  fn access_multi_visitor(&self, key: &Self::Key, visitor: &mut dyn FnMut(Self::Value)) {
    (**self).access_multi_visitor(key, visitor)
  }
}

impl<K: CKey, V: CKey> MultiQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  fn iter_keys(&self) -> impl Iterator<Item = K> + '_ {
    std::iter::empty()
  }

  fn access_multi(&self, _: &K) -> Option<impl Iterator<Item = V> + '_> {
    None::<std::iter::Empty<V>>
  }
}

impl<K: CKey, V: CValue> MultiQuery for FastHashMap<K, FastHashSet<V>> {
  type Key = K;
  type Value = V;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.keys().cloned()
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    self.get(key).map(|set| set.iter().cloned())
  }
}

impl<T: MultiQuery> MultiQuery for LockReadGuardHolder<T> {
  type Key = T::Key;
  type Value = T::Value;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    (**self).iter_keys()
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    (**self).access_multi(key)
  }
}

pub fn validate_multi_query_consistency<Q: MultiQuery>(query: &Q) {
  let keys: Vec<Q::Key> = query.iter_keys().collect();

  // verify no duplicate keys
  let unique_keys: FastHashSet<_> = keys.iter().collect();
  assert_eq!(
    unique_keys.len(),
    keys.len(),
    "iter_keys should not return duplicate keys"
  );

  // verify each key from iter_keys has at least one value via access_multi
  for key in &keys {
    let values: Vec<_> = query
      .access_multi(key)
      .expect("access_multi should return Some for keys from iter_keys")
      .collect();
    assert!(
      !values.is_empty(),
      "access_multi should return non-empty iterator for key {:?}",
      key
    );
  }
}

#[test]
fn test_fast_hash_map_multi_query() {
  let mut map: FastHashMap<u32, FastHashSet<String>> = FastHashMap::default();
  map.insert(1, FastHashSet::from_iter(["a".to_string(), "b".to_string()]));
  map.insert(2, FastHashSet::from_iter(["c".to_string()]));

  validate_multi_query_consistency(&map);

  let v1: FastHashSet<_> = map.access_multi(&1).unwrap().collect();
  assert_eq!(v1.len(), 2);
  assert!(v1.contains(&"a".to_string()));
  assert!(v1.contains(&"b".to_string()));

  let v2: FastHashSet<_> = map.access_multi(&2).unwrap().collect();
  assert_eq!(v2.len(), 1);
  assert!(v2.contains(&"c".to_string()));

  assert!(map.access_multi(&3).is_none());
}
