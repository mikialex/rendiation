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
