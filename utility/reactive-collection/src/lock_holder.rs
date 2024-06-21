use crate::*;

pub(crate) trait MakeLockResultHolder<K, V>: Sized {
  /// note, this method should be considered as the unsafe
  fn make_lock_holder_collection(&self) -> Box<dyn DynVirtualCollection<K, V>>;
}

impl<T, K: CKey, V: CValue> MakeLockResultHolder<K, V> for Arc<RwLock<T>>
where
  T: VirtualCollection<K, V> + 'static,
{
  fn make_lock_holder_collection(&self) -> Box<dyn DynVirtualCollection<K, V>> {
    Box::new(self.make_read_holder())
  }
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V>
  for LockReadGuardHolder<T>
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    (**self).iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    (**self).access(key)
  }
}

impl<K, V, T> VirtualCollectionSelfContained<K, V> for LockReadGuardHolder<T>
where
  K: CKey,
  V: CValue,
  T: VirtualCollection<K, V> + VirtualCollectionSelfContained<K, V>,
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.deref().access_ref(key)
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
