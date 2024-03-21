use crate::*;

pub struct EntityComponentGroupExclusiveLock {
  _locks: Vec<Box<dyn Any>>,
}
pub struct EntityComponentGroupShareLock {
  _locks: Vec<Box<dyn Any>>,
}

impl EntityComponentGroup {
  pub fn lock_exclusive(&self) -> EntityComponentGroupExclusiveLock {
    let components = self.inner.components.read_recursive();
    EntityComponentGroupExclusiveLock {
      _locks: components
        .values()
        .map(|c| c.inner.create_write_holder())
        .collect(),
    }
  }
  pub fn lock_shared(&self) -> EntityComponentGroupShareLock {
    let components = self.inner.components.read_recursive();
    EntityComponentGroupShareLock {
      _locks: components
        .values()
        .map(|c| c.inner.create_read_holder())
        .collect(),
    }
  }
}

pub struct DatabaseExclusiveLock {
  _locks: Vec<EntityComponentGroupExclusiveLock>,
}

pub struct DatabaseShareLock {
  _locks: Vec<EntityComponentGroupShareLock>,
}

impl Database {
  pub fn lock_exclusive(&self) -> DatabaseExclusiveLock {
    let tables = self.ecg_tables.read();
    DatabaseExclusiveLock {
      _locks: tables.values().map(|ecg| ecg.lock_exclusive()).collect(),
    }
  }
  pub fn lock_shared(&self) -> DatabaseShareLock {
    let tables = self.ecg_tables.read();
    DatabaseShareLock {
      _locks: tables.values().map(|ecg| ecg.lock_shared()).collect(),
    }
  }
}
