use crate::*;

pub trait VirtualMultiCollectionExt<K: CKey, V: CValue>:
  VirtualMultiCollection<K, V> + Sized + 'static
{
  fn into_boxed(self) -> Box<dyn VirtualMultiCollection<K, V>> {
    Box::new(self)
  }

  fn map<V2: CValue>(
    self,
    mapper: impl Fn(V) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K, V2> {
    MappedMultiCollection {
      base: self.into_boxed(),
      mapper,
    }
  }

  fn key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  ) -> impl VirtualMultiCollection<K2, V> {
    KeyDualMapMultiCollection {
      base: self.into_boxed(),
      f1,
      f2,
    }
  }
}
impl<T: ?Sized, K: CKey, V: CValue> VirtualMultiCollectionExt<K, V> for T where
  Self: VirtualMultiCollection<K, V> + Sized + 'static
{
}

#[derive(Clone)]
pub struct MappedMultiCollection<'a, K, V, F> {
  pub base: Box<dyn VirtualMultiCollection<K, V> + 'a>,
  pub mapper: F,
}

impl<'a, K, V, V2, F> VirtualMultiCollection<K, V2> for MappedMultiCollection<'a, K, V, F>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(V) -> V2 + Clone + Send + Sync + 'static,
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    self.base.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<Box<dyn Iterator<Item = V2> + '_>> {
    Some(Box::new(
      self.base.access_multi(key)?.map(|v| (self.mapper)(v)),
    ))
  }
}

#[derive(Clone)]
pub struct KeyDualMapMultiCollection<'a, K, V, F1, F2> {
  pub base: Box<dyn VirtualMultiCollection<K, V> + 'a>,
  pub f1: F1,
  pub f2: F2,
}

impl<'a, K, K2, V, F1, F2> VirtualMultiCollection<K2, V>
  for KeyDualMapMultiCollection<'a, K, V, F1, F2>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K2> + '_> {
    Box::new(
      self
        .base
        .iter_key_in_multi_collection()
        .map(|k| (self.f1)(k)),
    )
  }

  fn access_multi(&self, key: &K2) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    let k = (self.f2)(key.clone())?;
    self.base.access_multi(&k)
  }
}
