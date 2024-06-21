use crate::*;

pub trait DynVirtualCollection<K, V>: DynClone + Send + Sync {
  fn iter_key_value_dyn(&self) -> Box<dyn Iterator<Item = (K, V)> + '_>;
  fn access_dyn(&self, key: &K) -> Option<V>;
}
impl<K: CKey, V: CValue, T> DynVirtualCollection<K, V> for T
where
  T: VirtualCollection<K, V>,
{
  fn iter_key_value_dyn(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new(self.iter_key_value())
  }
  fn access_dyn(&self, key: &K) -> Option<V> {
    self.access(key)
  }
}

impl<'a, K, V> Clone for Box<dyn DynVirtualCollection<K, V> + 'a> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for Box<dyn DynVirtualCollection<K, V> + 'a> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access_dyn(key)
  }
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for &'a dyn DynVirtualCollection<K, V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (*self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (*self).access_dyn(key)
  }
}
