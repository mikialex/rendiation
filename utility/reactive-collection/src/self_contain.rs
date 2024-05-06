use crate::*;

pub trait ReactiveCollectionSelfContained<K: CKey, V: CValue>: ReactiveCollection<K, V> {
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V> + '_>;

  fn self_contain_into_boxed(self) -> Box<dyn ReactiveCollectionSelfContained<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}

pub trait IntoReactiveCollectionSelfContainedExt<K: CKey, V: CValue>:
  ReactiveCollection<K, V>
{
  fn into_self_contained(self) -> impl ReactiveCollectionSelfContained<K, V>;
}

impl<T, K: CKey, V: CValue> IntoReactiveCollectionSelfContainedExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
{
  fn into_self_contained(self) -> impl ReactiveCollectionSelfContained<K, V> {
    todo!()
  }
}

pub trait VirtualCollectionSelfContained<K: CKey, V: CValue>: VirtualCollection<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V>;
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V>
  for &'a dyn VirtualCollectionSelfContained<K, V>
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    (**self).iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access(key)
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

impl<K: CKey, V: CValue> ReactiveCollectionSelfContained<K, V> for () {
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V> + '_> {
    Box::new(())
  }
}

impl<K: CKey, V: CValue> VirtualCollectionSelfContained<K, V> for FastHashMap<K, V> {
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.get(key)
  }
}

impl<K: CKey, V: CValue, T: VirtualCollectionSelfContained<K, V>>
  VirtualCollectionSelfContained<K, V> for LockReadGuardHolder<T>
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.deref().access_ref(key)
  }
}
