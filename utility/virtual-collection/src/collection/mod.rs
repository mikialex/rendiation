use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

mod self_contain;
pub use self_contain::*;

pub trait VirtualCollection: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
  fn access(&self, key: &Self::Key) -> Option<Self::Value>;
  fn contains(&self, key: &Self::Key) -> bool {
    self.access(key).is_some()
  }

  fn materialize(&self) -> Arc<FastHashMap<Self::Key, Self::Value>> {
    Arc::new(self.iter_key_value().collect())
  }
  fn materialize_hashmap_maybe_cloned(&self) -> FastHashMap<Self::Key, Self::Value> {
    Arc::try_unwrap(self.materialize()).unwrap_or_else(|m| m.deref().clone())
  }
}

impl<'a, T: VirtualCollection> VirtualCollection for &'a T {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (*self).iter_key_value()
  }

  fn access(&self, k: &Self::Key) -> Option<Self::Value> {
    (*self).access(k)
  }
}

pub struct EmptyCollection<K, V>(PhantomData<(K, V)>);

impl<K, V> Clone for EmptyCollection<K, V> {
  fn clone(&self) -> Self {
    Self(self.0)
  }
}

impl<K, V> Default for EmptyCollection<K, V> {
  fn default() -> Self {
    Self(PhantomData)
  }
}

impl<K: CKey, V: CValue> VirtualCollection for EmptyCollection<K, V> {
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    std::iter::empty()
  }

  fn access(&self, _: &K) -> Option<V> {
    None
  }
}

impl<K: CKey, V: CValue> VirtualCollection for Arc<FastHashMap<K, V>> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|(k, v)| (k.clone(), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
  fn materialize(&self) -> Arc<FastHashMap<K, V>> {
    self.clone()
  }
}

impl<K: CKey, V: CValue> VirtualCollection for FastHashMap<K, V> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|(k, v)| (k.clone(), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
}

impl<K: CKey, V: CValue> VirtualCollection for dashmap::DashMap<K, V, FastHasherBuilder> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|v| (v.key().clone(), v.value().clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key)?.value().clone().into()
  }
}

impl<V: CValue> VirtualCollection for Arena<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(h, v)| (h.index() as u32, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    let handle = self.get_handle(*key as usize).unwrap();
    self.get(handle).cloned()
  }
}

impl<V: CValue> VirtualCollection for IndexReusedVec<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(k, v)| (k, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(*key).cloned()
  }
}

impl<V: CValue> VirtualCollection for IndexKeptVec<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(k, v)| (k, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(key.alloc_index()).cloned()
  }
}
