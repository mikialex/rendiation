use crate::*;

pub trait MultiQueryExt: MultiQuery + Sized + 'static {
  fn into_boxed_multi(self) -> BoxedDynMultiQuery<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn multi_map<V2, F>(self, mapper: F) -> MappedQuery<Self, F>
  where
    V2: CValue,
    F: Fn(&Self::Key, Self::Value) -> V2 + Clone + Send + Sync + 'static,
  {
    MappedQuery { base: self, mapper }
  }

  fn multi_key_dual_map<K2, F1, F2>(
    self,
    f1: F1,
    f2: F2,
  ) -> KeyDualMappedQuery<Self, F1, AutoSomeFnResult<F2>>
  where
    K2: CKey,
    F1: Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    F2: Fn(K2) -> Self::Key + Clone + Send + Sync + 'static,
  {
    self.multi_key_dual_map_partial(f1, AutoSomeFnResult(f2))
  }

  fn multi_key_dual_map_partial<K2, F1, F2>(
    self,
    f1: F1,
    f2: F2,
  ) -> KeyDualMappedQuery<Self, F1, F2>
  where
    K2: CKey,
    F1: Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    F2: Fn(K2) -> Option<Self::Key> + Clone + Send + Sync + 'static,
  {
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

impl<K2, F1, F2, T> MultiQuery for KeyDualMappedQuery<T, F1, F2>
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
