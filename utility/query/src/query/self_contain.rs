use crate::*;

pub type BoxedDynValueRefQuery<K, V> = Box<dyn DynValueRefQuery<Key = K, Value = V>>;
pub trait DynValueRefQuery: DynQuery {
  fn access_ref(&self, key: &Self::Key) -> Option<&Self::Value>;
}

impl<K: CKey, V: CValue> Query for &dyn DynValueRefQuery<Key = K, Value = V> {
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access_dyn(key)
  }
}

impl<K: CKey, V: CValue> DynValueRefQuery for &dyn DynValueRefQuery<Key = K, Value = V> {
  fn access_ref(&self, key: &K) -> Option<&V> {
    (**self).access_ref(key)
  }
}

impl<K: CKey, V: CValue> DynValueRefQuery for EmptyQuery<K, V> {
  fn access_ref(&self, _: &K) -> Option<&V> {
    None
  }
}

impl<K: CKey, V: CValue> DynValueRefQuery for FastHashMap<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.get(key)
  }
}

impl<V: CValue> DynValueRefQuery for IndexKeptVec<V> {
  fn access_ref(&self, key: &u32) -> Option<&V> {
    self.try_get(key.alloc_index())
  }
}
