use crate::*;

pub trait DynVirtualMultiCollection<K, V: CValue>: Send + Sync + DynClone {
  fn iter_key_in_multi_collection_dyn(&self) -> Box<dyn Iterator<Item = K> + '_>;
  /// if k is not in the collection at all, return None.
  /// if k is in the collection but map to none of v, return empty iterator
  fn access_multi_dyn(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>>;
  fn access_multi_value_dyn(&self, key: &K) -> Box<dyn Iterator<Item = V> + '_> {
    self
      .access_multi_dyn(key)
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor_dyn(&self, key: &K, visitor: &mut dyn FnMut(V)) {
    if let Some(v) = self.access_multi_dyn(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

impl<K: CKey, V: CValue, T> DynVirtualMultiCollection<K, V> for T
where
  T: VirtualMultiCollection<K, V>,
{
  fn iter_key_in_multi_collection_dyn(&self) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new(self.iter_key_in_multi_collection())
  }

  fn access_multi_dyn(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    self
      .access_multi(key)
      .map(|v| Box::new(v) as Box<dyn Iterator<Item = V> + '_>)
  }
}

impl<'a, K, V> Clone for Box<dyn DynVirtualMultiCollection<K, V> + 'a> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<'a, K: CKey, V: CKey> VirtualMultiCollection<K, V>
  for Box<dyn DynVirtualMultiCollection<K, V> + 'a>
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    (**self).iter_key_in_multi_collection_dyn()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_> {
    (**self).access_multi_dyn(key)
  }
}

impl<'a, K: CKey, V: CKey> VirtualMultiCollection<K, V>
  for &'a dyn DynVirtualMultiCollection<K, V>
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    (**self).iter_key_in_multi_collection_dyn()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_> {
    (**self).access_multi_dyn(key)
  }
}
