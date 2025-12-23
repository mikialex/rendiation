use crate::*;

pub struct EntityComponentGroupExclusiveLock {
  _locks: EntityWriterUntyped,
}
pub struct EntityComponentGroupShareLock {
  _locks: EntityReaderUntyped,
}

impl ArcTable {
  pub fn lock_exclusive(&self) -> EntityComponentGroupExclusiveLock {
    EntityComponentGroupExclusiveLock {
      _locks: self.entity_writer_dyn(),
    }
  }
  pub fn lock_shared(&self) -> EntityComponentGroupShareLock {
    EntityComponentGroupShareLock {
      _locks: self.entity_reader_dyn(),
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
    let tables = self.tables.read();
    DatabaseExclusiveLock {
      _locks: tables.values().map(|t| t.lock_exclusive()).collect(),
    }
  }
  pub fn lock_shared(&self) -> DatabaseShareLock {
    let tables = self.tables.read();
    DatabaseShareLock {
      _locks: tables.values().map(|t| t.lock_shared()).collect(),
    }
  }
}
