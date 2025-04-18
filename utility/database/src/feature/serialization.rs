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
}

impl ComponentCollectionUntyped {
  pub fn serialize(&self) -> DatabaseSerializationComponentResult {
    let data = self.read_untyped().data.fast_serialize_all();
    DatabaseSerializationComponentResult { data }
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
      .into_iter()
      .map(|(k, v)| (k, v.end_tracing()))
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
  pub fn end_tracing(self) -> DatabaseTraceECGResult {
    DatabaseTraceECGResult {
      components: self
        .components
        .into_iter()
        .map(|(k, v)| (k, v.end_tracing()))
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
  data: Arc<RwLock<Vec<u8>>>,
  event_remover: RemoveToken<ChangePtr>,
  event: EventSource<ChangePtr>,
}

impl ComponentCollectionUntyped {
  pub fn start_tracing(&self) -> DatabaseTraceComponentController {
    let event_remover = self.data_watchers.on(move |_change| {
      // let change = (*change) as *const ScopedValueChange<T>;
      // let change = &*change as &ScopedValueChange<T>;
      // match change {
      //   ScopedMessage::Start => {
      //     sender.lock();
      //     false
      //   }
      //   ScopedMessage::End => {
      //     sender.unlock();
      //     sender.is_closed()
      //   }
      //   ScopedMessage::Message(write) => {
      //     sender.send(write.idx, write.change.clone());
      //     false
      //   }
      // }

      false
    });
    DatabaseTraceComponentController {
      data: Default::default(),
      event_remover,
      event: self.data_watchers.clone(),
    }
  }
}

impl DatabaseTraceComponentController {
  pub fn end_tracing(self) -> DatabaseTraceComponentResult {
    self.event.off(self.event_remover);
    let data = Arc::try_unwrap(self.data).unwrap();
    let data = data.into_inner();
    DatabaseTraceComponentResult { data }
  }
}
