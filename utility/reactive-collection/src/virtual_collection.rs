use std::{ops::DerefMut, sync::Arc};

use dyn_clone::DynClone;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
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

impl<T, K: CKey, V: CValue> MakeLockResultHolder<K, V> for Arc<RwLock<T>>
where
  T: VirtualCollection<K, V> + 'static,
{
  fn make_lock_holder_collection(&self) -> Box<dyn VirtualCollection<K, V>> {
    Box::new(self.make_read_holder())
  }
}

pub trait MakeLockResultHolderRaw<T>: Sized {
  fn make_read_holder(&self) -> LockReadGuardHolder<T>;
  fn make_write_holder(&self) -> LockWriteGuardHolder<T>;
}
impl<T> MakeLockResultHolderRaw<T> for Arc<RwLock<T>> {
  fn make_read_holder(&self) -> LockReadGuardHolder<T> {
    let lock = self.read_recursive();
    let lock: RwLockReadGuard<'static, T> = unsafe { std::mem::transmute(lock) };
    LockReadGuardHolder {
      holder: self.clone(),
      guard: Arc::new(lock),
    }
  }

  fn make_write_holder(&self) -> LockWriteGuardHolder<T> {
    let lock = self.write();
    let lock: RwLockWriteGuard<'static, T> = unsafe { std::mem::transmute(lock) };
    LockWriteGuardHolder {
      holder: self.clone(),
      guard: lock,
    }
  }
}

pub trait MakeMutexHolderRaw<T>: Sized {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T>;
}

impl<T> MakeMutexHolderRaw<T> for Arc<std::sync::Mutex<T>> {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T> {
    let lock = self.lock().unwrap();
    let lock: std::sync::MutexGuard<'static, T> = unsafe { std::mem::transmute(lock) };
    MutexGuardHolder {
      _holder: self.clone(),
      guard: lock,
    }
  }
}

/// Note, the field(drop) order is important
pub struct MutexGuardHolder<T: 'static> {
  guard: std::sync::MutexGuard<'static, T>,
  _holder: Arc<std::sync::Mutex<T>>,
}

impl<T: 'static> Deref for MutexGuardHolder<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.guard.deref()
  }
}

impl<T: 'static> DerefMut for MutexGuardHolder<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.guard.deref_mut()
  }
}

/// Note, the field(drop) order is important
pub struct LockReadGuardHolder<T: 'static> {
  guard: Arc<RwLockReadGuard<'static, T>>,
  holder: Arc<RwLock<T>>,
}

impl<T: 'static> LockReadGuardHolder<T> {
  pub fn get_lock(&self) -> Arc<RwLock<T>> {
    self.holder.clone()
  }
}

impl<T: 'static> Deref for LockReadGuardHolder<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.guard.deref()
  }
}

impl<T: 'static> Clone for LockReadGuardHolder<T> {
  fn clone(&self) -> Self {
    Self {
      guard: self.guard.clone(),
      holder: self.holder.clone(),
    }
  }
}

/// Note, the field(drop) order is important
pub struct LockWriteGuardHolder<T: 'static> {
  guard: RwLockWriteGuard<'static, T>,
  holder: Arc<RwLock<T>>,
}

impl<T: 'static> LockWriteGuardHolder<T> {
  pub fn get_lock(&self) -> Arc<RwLock<T>> {
    self.holder.clone()
  }
}

impl<T: 'static> Deref for LockWriteGuardHolder<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.guard.deref()
  }
}

impl<T: 'static> DerefMut for LockWriteGuardHolder<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.guard.deref_mut()
  }
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V>
  for LockReadGuardHolder<T>
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    self.guard.iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    self.guard.access(key)
  }
}

#[derive(Clone)]
pub struct GeneralVirtualCollection<'a, K, V> {
  pub access: Arc<dyn Fn(&K) -> Option<V> + 'a + Send + Sync>,
  pub make_iter: Arc<dyn Fn() -> Box<dyn Iterator<Item = (K, V)> + 'a> + Send + Sync + 'a>,
}

// pub fn impl_virtual_collection<'a, K: CKey, V: CValue>(
//   access: impl Fn(&K) -> Option<V> + Send + Sync + 'a,
//   make_iter: impl Fn() -> Box<dyn Iterator<Item = (K, V)>> + Send + Sync + 'a,
// ) -> Box<dyn VirtualCollection<K, V> + 'a> {
//   Box::new(GeneralVirtualCollection {
//     access: Arc::new(access),
//     make_iter: Arc::new(make_iter),
//   })
// }

impl<'a, K, V> VirtualCollection<K, V> for GeneralVirtualCollection<'a, K, V>
where
  K: CKey,
  V: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    (self.make_iter)()
  }

  fn access(&self, key: &K) -> Option<V> {
    (self.access)(key)
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
