use crate::*;

pub type BoxedDynMultiQuery<K, V> = Box<dyn DynMultiQuery<Key = K, Value = V>>;
pub trait DynMultiQuery: Send + Sync + DynClone {
  type Key: CKey;
  type Value: CValue;
  fn iter_keys_dyn(&self) -> Box<dyn Iterator<Item = Self::Key> + '_>;
  /// if k is not in the query at all, return None.
  /// if k is in the query but map to none of v, return empty iterator
  fn access_multi_dyn(&self, key: &Self::Key)
    -> Option<Box<dyn Iterator<Item = Self::Value> + '_>>;
  fn access_multi_value_dyn(&self, key: &Self::Key) -> Box<dyn Iterator<Item = Self::Value> + '_> {
    self
      .access_multi_dyn(key)
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor_dyn(&self, key: &Self::Key, visitor: &mut dyn FnMut(Self::Value)) {
    if let Some(v) = self.access_multi_dyn(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

impl<T> DynMultiQuery for T
where
  T: MultiQuery,
{
  type Key = T::Key;
  type Value = T::Value;
  fn iter_keys_dyn(&self) -> Box<dyn Iterator<Item = T::Key> + '_> {
    Box::new(self.iter_keys())
  }

  fn access_multi_dyn(&self, key: &T::Key) -> Option<Box<dyn Iterator<Item = T::Value> + '_>> {
    self
      .access_multi(key)
      .map(|v| Box::new(v) as Box<dyn Iterator<Item = T::Value> + '_>)
  }
}

impl<K, V> Clone for Box<dyn DynMultiQuery<Key = K, Value = V> + '_> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<K: CKey, V: CKey> MultiQuery for Box<dyn DynMultiQuery<Key = K, Value = V> + '_> {
  type Key = K;
  type Value = V;
  fn iter_keys(&self) -> impl Iterator<Item = K> + '_ {
    (**self).iter_keys_dyn()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_> {
    (**self).access_multi_dyn(key)
  }
}

impl<K: CKey, V: CKey> MultiQuery for &dyn DynMultiQuery<Key = K, Value = V> {
  type Key = K;
  type Value = V;
  fn iter_keys(&self) -> impl Iterator<Item = K> + '_ {
    (**self).iter_keys_dyn()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_> {
    (**self).access_multi_dyn(key)
  }
}
