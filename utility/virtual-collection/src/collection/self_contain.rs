use crate::*;

pub type BoxedDynVirtualCollectionSelfContained<K, V> =
  Box<dyn DynVirtualCollectionSelfContained<Key = K, Value = V>>;
pub trait DynVirtualCollectionSelfContained: DynVirtualCollection {
  fn access_ref(&self, key: &Self::Key) -> Option<&Self::Value>;
}

impl<'a, K: CKey, V: CValue> VirtualCollection
  for &'a dyn DynVirtualCollectionSelfContained<Key = K, Value = V>
{
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value_dyn()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access_dyn(key)
  }
}

impl<'a, K: CKey, V: CValue> DynVirtualCollectionSelfContained
  for &'a dyn DynVirtualCollectionSelfContained<Key = K, Value = V>
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    (**self).access_ref(key)
  }
}

impl<K: CKey, V: CValue> DynVirtualCollectionSelfContained for EmptyCollection<K, V> {
  fn access_ref(&self, _: &K) -> Option<&V> {
    None
  }
}

impl<K: CKey, V: CValue> DynVirtualCollectionSelfContained for FastHashMap<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.get(key)
  }
}

impl<V: CValue> DynVirtualCollectionSelfContained for IndexKeptVec<V> {
  fn access_ref(&self, key: &u32) -> Option<&V> {
    self.try_get(key.alloc_index())
  }
}
