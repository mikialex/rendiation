use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

mod self_contain;
pub use self_contain::*;

mod container;
pub use container::*;

pub type QueryMaterialized<K, V> = FastHashMap<K, V>;

pub trait Query: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
  fn access(&self, key: &Self::Key) -> Option<Self::Value>;
  fn contains(&self, key: &Self::Key) -> bool {
    self.access(key).is_some()
  }

  /// the implementation allows to have false positive(return has item but is actually empty)
  fn has_item_hint(&self) -> bool;

  fn materialize(&self) -> Arc<QueryMaterialized<Self::Key, Self::Value>> {
    Arc::new(self.iter_key_value().collect())
  }

  /// this impl use iter hint size's upper bound for collection.
  /// as the iter likely has correct upper bound, using this materialize fn should avoid rehash and grow effectively
  fn materialize_upper_bound(&self) -> Arc<QueryMaterialized<Self::Key, Self::Value>> {
    let iter = self.iter_key_value();
    let size_hint = iter.size_hint();
    let size_pre_allocate = size_hint.1.unwrap_or(size_hint.0);
    let mut map = FastHashMap::with_capacity_and_hasher(size_pre_allocate, Default::default());
    for (k, v) in iter {
      map.insert(k, v);
    }

    if map.capacity() > 128 && map.capacity() > map.len() * 4 {
      map.shrink_to_fit();
    }

    Arc::new(map)
  }
}

impl<T: Query> Query for &T {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (*self).iter_key_value()
  }

  fn access(&self, k: &Self::Key) -> Option<Self::Value> {
    (*self).access(k)
  }

  fn has_item_hint(&self) -> bool {
    (*self).has_item_hint()
  }
}

impl<T: Query> Query for Option<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.iter().flat_map(|v| v.iter_key_value())
  }
  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.as_ref().and_then(|v| v.access(key))
  }

  fn has_item_hint(&self) -> bool {
    self.as_ref().is_some_and(|v| v.has_item_hint())
  }
}
