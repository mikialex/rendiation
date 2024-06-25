use crate::*;

pub trait VirtualMultiCollectionExt<K: CKey, V: CValue>:
  VirtualMultiCollection<K, V> + Sized + 'static
{
  fn into_boxed(self) -> Box<dyn DynVirtualMultiCollection<K, V>> {
    Box::new(self)
  }

  fn multi_map<V2: CValue>(
    self,
    mapper: impl Fn(&K, V) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K, V2> {
    MappedCollection {
      base: self,
      mapper,
      phantom: PhantomData,
    }
  }

  fn multi_key_dual_map<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> K + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K2, V> {
    self.multi_key_dual_map_partial(f1, move |k| Some(f2(k)))
  }

  fn multi_key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K2, V> {
    KeyDualMapCollection {
      base: self,
      f1,
      f2,
      phantom: PhantomData,
    }
  }
}
impl<T: ?Sized, K: CKey, V: CValue> VirtualMultiCollectionExt<K, V> for T where
  Self: VirtualMultiCollection<K, V> + Sized + 'static
{
}

impl<K, V, V2, F, T> VirtualMultiCollection<K, V2> for MappedCollection<K, V, F, T>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(&K, V) -> V2 + Clone + Send + Sync + 'static,
  T: VirtualMultiCollection<K, V>,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    self.base.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V2> + '_> {
    let k = key.clone();
    Some(Box::new(
      self
        .base
        .access_multi(key)?
        .map(move |v| (self.mapper)(&k, v)),
    ))
  }
}

impl<K, K2, V, F1, F2, T> VirtualMultiCollection<K2, V> for KeyDualMapCollection<K, V, F1, F2, T>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  T: VirtualMultiCollection<K, V>,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K2> + '_ {
    Box::new(
      self
        .base
        .iter_key_in_multi_collection()
        .map(|k| (self.f1)(k)),
    )
  }

  fn access_multi(&self, key: &K2) -> Option<impl Iterator<Item = V> + '_> {
    let k = (self.f2)(key.clone())?;
    // I believe this is a compiler bug
    let k: &'static K = unsafe { std::mem::transmute(&k) };
    self.base.access_multi(k)
  }
}
