use crate::*;

impl Database {
  pub fn serialize(&self) -> (DatabaseSerializationResult, DatabaseShareLock) {
    let guard = self.lock_shared();

    let r = DatabaseSerializationResult {
      ecg: self
        .ecg_tables
        .read()
        .iter()
        .map(|(k, v)| (*k, v.serialize()))
        .collect(),
    };
    (r, guard)
  }
}

pub struct DatabaseSerializationResult {
  pub ecg: FastHashMap<EntityId, DatabaseSerializationECGResult>,
}

pub struct DatabaseSerializationECGResult {
  pub components: FastHashMap<ComponentId, DatabaseSerializationComponentResult>,
}

impl EntityComponentGroup {
  pub fn serialize(&self) -> DatabaseSerializationECGResult {
    DatabaseSerializationECGResult {
      components: self
        .inner
        .components
        .read()
        .iter()
        .map(|(k, v)| (*k, v.serialize()))
        .collect(),
    }
  }
}

pub struct DatabaseSerializationComponentResult {
  pub data: Vec<u8>,
  pub index: Vec<Option<usize>>,
}

impl ComponentCollectionUntyped {
  pub fn serialize(&self) -> DatabaseSerializationComponentResult {
    todo!()
  }
}

pub struct DatabaseMutationTracingController {
  db: Database,
  base_state: DatabaseSerializationResult,
  ecg: FastHashMap<EntityId, DatabaseTraceECGController>,
}

impl DatabaseMutationTracingController {
  pub fn record(db: &Database) -> Self {
    let (base_state, lock) = db.serialize(); // keep lock alive until we create all sub controller

    let ecg = db
      .ecg_tables
      .read()
      .iter()
      .map(|(k, v)| (*k, v.start_tracing()))
      .collect();

    drop(lock);

    Self {
      db: db.clone(),
      base_state,
      ecg,
    }
  }
  pub fn stop_record(self) -> DatabaseMutationTracingResult {
    let guard = self.db.lock_shared();

    let ecg = self
      .ecg
      .iter()
      .map(|(k, v)| (*k, v.end_tracing()))
      .collect();

    drop(guard);

    DatabaseMutationTracingResult {
      base_state: self.base_state,
      ecg,
    }
  }
}

pub struct DatabaseMutationTracingResult {
  pub base_state: DatabaseSerializationResult,
  pub ecg: FastHashMap<EntityId, DatabaseTraceECGResult>,
}

pub struct DatabaseTraceECGResult {
  pub components: FastHashMap<ComponentId, DatabaseTraceComponentResult>,
}

pub struct DatabaseTraceComponentResult {
  pub data: Vec<u8>,
}

pub struct DatabaseTraceECGController {
  pub components: FastHashMap<ComponentId, DatabaseTraceComponentController>,
}

impl DatabaseTraceECGController {
  pub fn end_tracing(&self) -> DatabaseTraceECGResult {
    DatabaseTraceECGResult {
      components: self
        .components
        .iter()
        .map(|(k, v)| (*k, v.end_tracing()))
        .collect(),
    }
  }
}

impl EntityComponentGroup {
  pub fn start_tracing(&self) -> DatabaseTraceECGController {
    DatabaseTraceECGController {
      components: self
        .inner
        .components
        .read()
        .iter()
        .map(|(k, v)| (*k, v.start_tracing()))
        .collect(),
    }
  }
}

pub struct DatabaseTraceComponentController {
  pub data: Vec<u8>,
}

impl ComponentCollectionUntyped {
  pub fn start_tracing(&self) -> DatabaseTraceComponentController {
    todo!()
  }
}

impl DatabaseTraceComponentController {
  pub fn end_tracing(&self) -> DatabaseTraceComponentResult {
    todo!()
  }
}
