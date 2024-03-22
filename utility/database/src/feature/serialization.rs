use std::path::Path;

use crate::*;

pub struct DatabaseSerialization {
  db: Database,
  //
}

pub struct DatabaseMutationTracing {
  db: Database,
  //
}

struct MutationTracingWatchHandle {
  ecg_meta_handle: usize,
  ecg_handles: Vec<EntityComponentGroupWatchHandle>,
}

struct EntityComponentGroupWatchHandle {
  component_meta_handle: usize,
  component_handles: Vec<usize>,
}

impl DatabaseMutationTracing {
  pub fn record(db: &Database, file_path: impl AsRef<Path>) -> Self {
    let guard = db.lock_shared();
    Self { db: db.clone() }
  }
  pub fn stop_record(self) {
    let guard = self.db.lock_shared();
  }
}

impl Drop for DatabaseMutationTracing {
  fn drop(&mut self) {
    todo!()
  }
  //
}
