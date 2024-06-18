use std::sync::Arc;

use dyn_clone::DynClone;
use storage::{Arena, IndexReusedVec};

use crate::*;

pub trait CKey: Eq + Hash + CValue {}
impl<T> CKey for T where T: Eq + Hash + CValue {}
pub trait CValue: Clone + Send + Sync + std::fmt::Debug + PartialEq + 'static {}
impl<T> CValue for T where T: Clone + Send + Sync + std::fmt::Debug + PartialEq + 'static {}

pub trait VirtualCollection<K: CKey, V: CValue>: Send + Sync + DynClone {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_>;
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
impl<'a, K, V> Clone for Box<dyn VirtualCollection<K, V> + 'a> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for Box<dyn VirtualCollection<K, V> + 'a> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    (**self).iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access(key)
  }
}

impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for &'a dyn VirtualCollection<K, V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    (*self).iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    (*self).access(key)
  }
}

pub trait VirtualCollectionExt<K: CKey, V: CValue>: VirtualCollection<K, V> {
  fn into_boxed(self) -> Box<dyn VirtualCollection<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }

  // fn values(&self) -> impl Iterator<Item = V> + '_ {
  //   self.iter_key_value().map(|(_, v)| v)
  // }
}
impl<T: ?Sized, K: CKey, V: CValue> VirtualCollectionExt<K, V> for T where
  Self: VirtualCollection<K, V>
{
}

pub trait VirtualMultiCollection<K, V: CValue>: Send + Sync {
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_>;
  /// if k is not in the collection at all, return None.
  /// if k is in the collection but map to none of v, return empty iterator
  fn access_multi(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>>;
  fn access_multi_value(&self, key: &K) -> Box<dyn Iterator<Item = V> + '_> {
    self
      .access_multi(key)
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor(&self, key: &K, visitor: &mut dyn FnMut(V)) {
    if let Some(v) = self.access_multi(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

impl<'a, K: CKey, V: CKey> VirtualMultiCollection<K, V>
  for Box<dyn VirtualMultiCollection<K, V> + 'a>
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    (**self).iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    (**self).access_multi(key)
  }
}

/// it's useful to use () as the empty collection
impl<K: CKey, V: CValue> VirtualCollection<K, V> for () {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new([].into_iter())
  }

  fn access(&self, _: &K) -> Option<V> {
    None
  }
}

/// it's useful to use () as the empty collection
impl<K: CKey, V: CKey> VirtualMultiCollection<K, V> for () {
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new([].into_iter())
  }

  fn access_multi(&self, _: &K) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    None
  }
}

impl<K: CKey, V: CValue> VirtualCollection<K, V> for Arc<FastHashMap<K, V>> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new(self.iter().map(|(k, v)| (k.clone(), v.clone())))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
  fn materialize(&self) -> Arc<FastHashMap<K, V>> {
    self.clone()
  }
}

impl<K: CKey, V: CValue> VirtualCollection<K, V> for FastHashMap<K, V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new(self.iter().map(|(k, v)| (k.clone(), v.clone())))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
}

impl<K: CKey, V: CValue> VirtualCollection<K, V> for dashmap::DashMap<K, V, FastHasherBuilder> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    Box::new(self.iter().map(|v| (v.key().clone(), v.value().clone())))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key)?.value().clone().into()
  }
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V>
  for LockReadGuardHolder<T>
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    (**self).iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access(key)
  }
}

impl<V: CValue> VirtualCollection<u32, V> for Arena<V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (u32, V)> + '_> {
    Box::new(self.iter().map(|(h, v)| (h.index() as u32, v.clone())))
  }

  fn access(&self, key: &u32) -> Option<V> {
    let handle = self.get_handle(*key as usize).unwrap();
    self.get(handle).cloned()
  }
}

impl<V: CValue> VirtualCollection<u32, V> for IndexReusedVec<V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (u32, V)> + '_> {
    Box::new(self.iter().map(|(k, v)| (k, v.clone())))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(*key).cloned()
  }
}
