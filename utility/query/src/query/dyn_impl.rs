use crate::*;

pub type BoxedDynQuery<K, V> = Box<dyn DynQuery<Key = K, Value = V>>;
pub trait DynQuery: DynClone + Send + Sync {
  type Key: CKey;
  type Value: CValue;
  fn iter_key_value_dyn(&self) -> Box<dyn Iterator<Item = (Self::Key, Self::Value)> + '_>;
  fn access_dyn(&self, key: &Self::Key) -> Option<Self::Value>;
}
impl<T> DynQuery for T
where
  T: Query,
{
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value_dyn(&self) -> Box<dyn Iterator<Item = (Self::Key, Self::Value)> + '_> {
    Box::new(self.iter_key_value())
  }
  fn access_dyn(&self, key: &Self::Key) -> Option<Self::Value> {
    self.access(key)
  }
}

impl<K, V> Clone for Box<dyn DynQuery<Key = K, Value = V> + '_> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<K: CKey, V: CValue> Query for Box<dyn DynQuery<Key = K, Value = V> + '_> {
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access_dyn(key)
  }
}

impl<K: CKey, V: CValue> Query for &dyn DynQuery<Key = K, Value = V> {
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (*self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (*self).access_dyn(key)
  }
}
