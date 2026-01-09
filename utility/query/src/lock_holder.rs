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

  fn has_item_hint(&self) -> bool {
    (**self).has_item_hint()
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
    let guard = self.read_arc_recursive();
    LockReadGuardHolder { guard }
  }

  fn make_write_holder(&self) -> LockWriteGuardHolder<T> {
    let guard = self.write_arc();
    LockWriteGuardHolder { guard }
  }
}

pub trait MakeMutexHolder<T>: Sized {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T>;
}

impl<T> MakeMutexHolder<T> for Arc<parking_lot::Mutex<T>> {
  fn make_mutex_write_holder(&self) -> MutexGuardHolder<T> {
    let guard = self.lock_arc();
    MutexGuardHolder { guard }
  }
}

pub struct MutexGuardHolder<T: 'static> {
  guard: ArcMutexGuard<RawMutex, T>,
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

pub struct LockReadGuardHolder<T: 'static> {
  guard: ArcRwLockReadGuard<RawRwLock, T>,
}

impl<T: 'static> LockReadGuardHolder<T> {
  pub fn get_lock(&self) -> Arc<RwLock<T>> {
    ArcRwLockReadGuard::rwlock(&self.guard).clone()
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
      guard: ArcRwLockReadGuard::rwlock(&self.guard).read_arc_recursive(),
    }
  }
}

pub struct LockWriteGuardHolder<T: 'static> {
  guard: ArcRwLockWriteGuard<RawRwLock, T>,
}

impl<T: 'static> LockWriteGuardHolder<T> {
  pub fn get_lock(&self) -> Arc<RwLock<T>> {
    ArcRwLockWriteGuard::rwlock(&self.guard).clone()
  }
  pub fn downgrade_to_read(self) -> LockReadGuardHolder<T> {
    let guard = ArcRwLockWriteGuard::downgrade(self.guard);
    LockReadGuardHolder { guard }
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
