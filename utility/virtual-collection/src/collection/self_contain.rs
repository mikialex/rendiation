use crate::*;

pub trait VirtualCollectionSelfContained<K: CKey, V: CValue>: DynVirtualCollection<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V>;
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V>
  for &'a dyn VirtualCollectionSelfContained<K, V>
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access_dyn(key)
  }
}

impl<'a, K: CKey, V: CValue> VirtualCollectionSelfContained<K, V>
  for &'a dyn VirtualCollectionSelfContained<K, V>
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    (**self).access_ref(key)
  }
}

impl<K: CKey, V: CValue> VirtualCollectionSelfContained<K, V> for () {
  fn access_ref(&self, _: &K) -> Option<&V> {
    None
  }
}

impl<K: CKey, V: CValue> VirtualCollectionSelfContained<K, V> for FastHashMap<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.get(key)
  }
}

impl<K: CKey + LinearIdentification, V: CValue> VirtualCollectionSelfContained<K, V>
  for IndexKeptVec<V>
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.try_get(key.alloc_index())
  }
}
