use crate::*;

pub trait MultiQueryExt: MultiQuery + Sized + 'static {
  fn into_boxed(self) -> BoxedDynMultiQuery<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn multi_map<V2: CValue>(
    self,
    mapper: impl Fn(&Self::Key, Self::Value) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl MultiQuery<Key = Self::Key, Value = V2> {
    MappedQuery { base: self, mapper }
  }

  fn multi_key_dual_map<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Self::Key + Clone + Send + Sync + 'static,
  ) -> impl MultiQuery<Key = K2, Value = Self::Value> {
    self.multi_key_dual_map_partial(f1, move |k| Some(f2(k)))
  }

  fn multi_key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<Self::Key> + Clone + Send + Sync + 'static,
  ) -> impl MultiQuery<Key = K2, Value = Self::Value> {
    KeyDualMappedQuery { base: self, f1, f2 }
  }
}
impl<T: ?Sized> MultiQueryExt for T where Self: MultiQuery + Sized + 'static {}

impl<V2, F, T> MultiQuery for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: MultiQuery,
{
  type Key = T::Key;
  type Value = V2;
  fn iter_keys(&self) -> impl Iterator<Item = T::Key> + '_ {
    self.base.iter_keys()
  }

  fn access_multi(&self, key: &T::Key) -> Option<impl Iterator<Item = V2> + '_> {
    let k = key.clone();
    Some(Box::new(
      self
        .base
        .access_multi(key)?
        .map(move |v| (self.mapper)(&k, v)),
    ))
  }
}

impl<K2, F1, F2, T> MultiQuery for KeyDualMappedQuery<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<T::Key> + Clone + Send + Sync + 'static,
  T: MultiQuery,
{
  type Key = K2;
  type Value = T::Value;
  fn iter_keys(&self) -> impl Iterator<Item = K2> + '_ {
    Box::new(self.base.iter_keys().map(|k| (self.f1)(k)))
  }

  fn access_multi(&self, key: &K2) -> Option<impl Iterator<Item = T::Value> + '_> {
    let k = (self.f2)(key.clone())?;
    // I believe this is a compiler bug
    let k: &'static T::Key = unsafe { std::mem::transmute(&k) };
    self.base.access_multi(k)
  }
}
