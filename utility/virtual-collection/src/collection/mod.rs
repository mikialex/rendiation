use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

mod self_contain;
pub use self_contain::*;

pub trait VirtualCollection<K: CKey, V: CValue>: Send + Sync + Clone {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_;
  fn access(&self, key: &K) -> Option<V>;
  fn contains(&self, key: &K) -> bool {
    self.access(key).is_some()
  }

  fn materialize(&self) -> Arc<FastHashMap<K, V>> {
    Arc::new(self.iter_key_value().collect())
  }
  fn materialize_hashmap_maybe_cloned(&self) -> FastHashMap<K, V> {
    Arc::try_unwrap(self.materialize()).unwrap_or_else(|m| m.deref().clone())
  }
}

impl<'a, K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V> for &'a T {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (*self).iter_key_value()
  }

  fn access(&self, k: &K) -> Option<V> {
    (*self).access(k)
  }
}

/// it's useful to use () as the empty collection
impl<K: CKey, V: CValue> VirtualCollection<K, V> for () {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    std::iter::empty()
  }

  fn access(&self, _: &K) -> Option<V> {
    None
  }
}

impl<K: CKey, V: CValue> VirtualCollection<K, V> for Arc<FastHashMap<K, V>> {
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

impl<K: CKey, V: CValue> VirtualCollection<K, V> for FastHashMap<K, V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|(k, v)| (k.clone(), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
}

impl<K: CKey, V: CValue> VirtualCollection<K, V> for dashmap::DashMap<K, V, FastHasherBuilder> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|v| (v.key().clone(), v.value().clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key)?.value().clone().into()
  }
}

impl<V: CValue> VirtualCollection<u32, V> for Arena<V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(h, v)| (h.index() as u32, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    let handle = self.get_handle(*key as usize).unwrap();
    self.get(handle).cloned()
  }
}

impl<V: CValue> VirtualCollection<u32, V> for IndexReusedVec<V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(k, v)| (k, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(*key).cloned()
  }
}

impl<K: CKey + LinearIdentification, V: CValue> VirtualCollection<K, V> for IndexKeptVec<V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self
      .iter()
      .map(|(k, v)| (K::from_alloc_index(k), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.try_get(key.alloc_index()).cloned()
  }
}
