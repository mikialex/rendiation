use std::ops::DerefMut;

use parking_lot::*;

use crate::*;

impl<T: Query> Query for LockReadGuardHolder<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (**self).iter_key_value()
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    (**self).access(key)
  }
}

impl<T, K, V> DynValueRefQuery for LockReadGuardHolder<T>
where
  T: DynValueRefQuery<Key = K, Value = V> + Query<Key = K, Value = V>,
{
  fn access_ref(&self, key: &K) -> Option<&V> {
    self.deref().access_ref(key)
  }
}

pub trait MakeLockResultHolder<T>: Sized {
  fn make_read_holder(&self) -> LockReadGuardHolder<T>;
  fn make_write_holder(&self) -> LockWriteGuardHolder<T>;
}
impl<T> MakeLockResultHolder<T> for Arc<RwLock<T>> {
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

pub trait MakeMutexHolder<T>: Sized {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T>;
}

impl<T> MakeMutexHolder<T> for Arc<parking_lot::Mutex<T>> {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T> {
    let lock = self.lock();
    let lock: parking_lot::MutexGuard<'static, T> = unsafe { std::mem::transmute(lock) };
    MutexGuardHolder {
      _holder: self.clone(),
      guard: lock,
    }
  }
}

/// Note, the field(drop) order is important
pub struct MutexGuardHolder<T: 'static> {
  guard: parking_lot::MutexGuard<'static, T>,
  _holder: Arc<parking_lot::Mutex<T>>,
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
  pub fn downgrade_to_read(self) -> LockReadGuardHolder<T> {
    LockReadGuardHolder {
      guard: Arc::new(RwLockWriteGuard::downgrade(self.guard)),
      holder: self.holder,
    }
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
