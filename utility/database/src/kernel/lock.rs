use crate::*;

pub struct TableExclusiveLock {
  _locks: TableWriterUntyped,
}
pub struct TableShareLock {
  _locks: TableReaderUntyped,
}

impl ArcTable {
  pub fn lock_exclusive(&self) -> TableExclusiveLock {
    TableExclusiveLock {
      _locks: self.entity_writer_dyn(),
    }
  }
  pub fn lock_shared(&self) -> TableShareLock {
    TableShareLock {
      _locks: self.entity_reader_dyn(),
    }
  }
}

pub struct DatabaseExclusiveLock {
  _locks: Vec<TableExclusiveLock>,
}

pub struct DatabaseShareLock {
  _locks: Vec<TableShareLock>,
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
