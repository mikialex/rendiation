use crate::*;

pub trait VirtualMultiCollectionExt<K: CKey, V: CValue>:
  VirtualMultiCollection<K, V> + Sized + 'static
{
  fn into_boxed(self) -> Box<dyn DynVirtualMultiCollection<K, V>> {
    Box::new(self)
  }

  fn map<V2: CValue>(
    self,
    mapper: impl Fn(V) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K, V2> {
    MappedMultiCollection {
      base: self,
      mapper,
      phantom: PhantomData,
    }
  }

  fn key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K2, V> {
    KeyDualMapMultiCollection {
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

#[derive(Clone)]
pub struct MappedMultiCollection<K, V, F, T> {
  pub base: T,
  pub mapper: F,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, V2, F, T> VirtualMultiCollection<K, V2> for MappedMultiCollection<K, V, F, T>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(V) -> V2 + Clone + Send + Sync + 'static,
  T: VirtualMultiCollection<K, V>,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    self.base.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V2> + '_> {
    Some(Box::new(
      self.base.access_multi(key)?.map(|v| (self.mapper)(v)),
    ))
  }
}

#[derive(Clone)]
pub struct KeyDualMapMultiCollection<K, V, F1, F2, T> {
  pub base: T,
  pub f1: F1,
  pub f2: F2,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, K2, V, F1, F2, T> VirtualMultiCollection<K2, V>
  for KeyDualMapMultiCollection<K, V, F1, F2, T>
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
