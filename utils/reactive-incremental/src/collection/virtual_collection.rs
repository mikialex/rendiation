use std::sync::Arc;

use dyn_clone::DynClone;
use parking_lot::RwLockReadGuard;

use crate::*;

pub trait CKey: Eq + Hash + CValue {}
impl<T> CKey for T where T: Eq + Hash + CValue {}
pub trait CValue: Clone + Send + Sync + 'static {}
impl<T> CValue for T where T: Clone + Send + Sync + 'static {}

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

  fn values(&self) -> impl Iterator<Item = V> + '_ {
    self.iter_key_value().map(|(_, v)| v)
  }
}
impl<T: ?Sized, K: CKey, V: CValue> VirtualCollectionExt<K, V> for T where
  Self: VirtualCollection<K, V>
{
}

pub trait VirtualMultiCollection<K, V> {
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_multi(&self, key: &K, visitor: &mut dyn FnMut(V));
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

pub(crate) trait MakeLockResultHolder<K, V>: Sized {
  /// note, this method should be considered as the unsafe
  fn make_lock_holder_collection(&self) -> Box<dyn VirtualCollection<K, V>>;
}

impl<T, K: CKey, V: CValue> MakeLockResultHolder<K, V> for RwLock<T>
where
  T: VirtualCollection<K, V> + 'static,
{
  fn make_lock_holder_collection(&self) -> Box<dyn VirtualCollection<K, V>> {
    Box::new(self.make_lock_holder_raw())
  }
}

pub(crate) trait MakeLockResultHolderRaw<T>: Sized {
  /// note, this method should be considered as the unsafe
  fn make_lock_holder_raw(&self) -> LockResultHolder<T>;
}
impl<T> MakeLockResultHolderRaw<T> for RwLock<T> {
  fn make_lock_holder_raw(&self) -> LockResultHolder<T> {
    let lock = self.read_recursive();
    let lock: RwLockReadGuard<'static, T> = unsafe { std::mem::transmute(lock) };
    LockResultHolder {
      guard: Arc::new(lock),
    }
  }
}

pub(crate) struct LockResultHolder<T: 'static> {
  guard: Arc<RwLockReadGuard<'static, T>>,
}

impl<T: 'static> Deref for LockResultHolder<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.guard.deref()
  }
}

impl<T: 'static> Clone for LockResultHolder<T> {
  fn clone(&self) -> Self {
    Self {
      guard: self.guard.clone(),
    }
  }
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V>
  for LockResultHolder<T>
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    self.guard.iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    self.guard.access(key)
  }
}
