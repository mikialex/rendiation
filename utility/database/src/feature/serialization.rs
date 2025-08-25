use std::{io::Write, thread::JoinHandle};

use futures::{channel::mpsc::UnboundedSender, StreamExt};
use parking_lot::lock_api::RawRwLock;

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
  pub allocator: Arena<()>,
  pub components: FastHashMap<ComponentId, DatabaseSerializationComponentResult>,
}

impl EntityComponentGroup {
  pub fn serialize(&self) -> DatabaseSerializationECGResult {
    DatabaseSerializationECGResult {
      allocator: self.inner.allocator.read().clone(),
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
  write_finish_handle: JoinHandle<()>,
}

impl DatabaseMutationTracingController {
  pub fn record(db: &Database) -> Self {
    let (base_state, lock) = db.serialize(); // keep lock alive until we create all sub controller

    let (sender, mut receiver) = futures::channel::mpsc::unbounded::<Vec<u8>>();

    // we should get a thread pool
    let write_finish_handle = std::thread::spawn(move || {
      let mut file = std::fs::File::create("db_trace.bin").unwrap();

      while let Some(data) = futures::executor::block_on(receiver.next()) {
        if data.is_empty() {
          continue;
        }
        println!("write {:?}", data);
        file.write_all(&data).unwrap();
      }
      file.flush().unwrap();
    });

    let ecg = db
      .ecg_tables
      .read()
      .iter()
      .map(|(k, v)| (*k, v.start_tracing(sender.clone())))
      .collect();

    drop(lock);

    Self {
      db: db.clone(),
      base_state,
      write_finish_handle,
      ecg,
    }
  }

  #[must_use]
  pub fn stop_record(self) -> DatabaseMutationTracingResult {
    let guard = self.db.lock_shared();

    self.ecg.into_iter().for_each(|(_, v)| v.end_tracing());

    drop(guard);

    DatabaseMutationTracingResult {
      base_state: self.base_state,
      write_finish_handle: self.write_finish_handle,
    }
  }
}

pub struct DatabaseMutationTracingResult {
  pub base_state: DatabaseSerializationResult,
  pub write_finish_handle: JoinHandle<()>,
}

pub struct DatabaseTraceECGController {
  pub components: FastHashMap<ComponentId, DatabaseTraceComponentController>,
}

impl DatabaseTraceECGController {
  pub fn end_tracing(self) {
    self
      .components
      .into_iter()
      .for_each(|(_, v)| v.end_tracing());
  }
}

impl EntityComponentGroup {
  pub fn start_tracing(&self, sender: UnboundedSender<Vec<u8>>) -> DatabaseTraceECGController {
    DatabaseTraceECGController {
      components: self
        .inner
        .components
        .read()
        .iter()
        .map(|(k, v)| (*k, v.start_tracing(sender.clone())))
        .collect(),
    }
  }
}

pub struct DatabaseTraceComponentController {
  event_remover: RemoveToken<ChangePtr>,
  event: EventSource<ChangePtr>,
}

impl ComponentCollectionUntyped {
  pub fn start_tracing(
    &self,
    sender: UnboundedSender<Vec<u8>>,
  ) -> DatabaseTraceComponentController {
    let data: Arc<RwLock<Option<Vec<u8>>>> = Default::default();
    let event_remover = self.data_watchers.on(move |change| unsafe {
      match change {
        ScopedMessage::Start => {
          data.raw().lock_exclusive();
          let data = &mut *data.data_ptr();
          *data = Some(Vec::default())
        }
        ScopedMessage::End => {
          {
            let data = &mut *data.data_ptr();
            let data = data.take().unwrap();
            sender.unbounded_send(data).unwrap();
          }

          data.raw().unlock_exclusive();
        }
        ScopedMessage::Message(write) => {
          let data = &mut *data.data_ptr();
          let data = data.as_mut().unwrap();
          match &write.change {
            ValueChange::Delta(new, old) => {
              if old.is_none() {
                data.push(1);
              } else {
                data.push(2);
              }
              data.extend_from_slice(bytes_of(&write.idx));
              (*new.1).fast_serialize_dyn(data);
            }
            ValueChange::Remove(_) => {
              data.push(0);
              data.extend_from_slice(bytes_of(&write.idx))
            }
          }
        }
      }

      false
    });
    DatabaseTraceComponentController {
      event_remover,
      event: self.data_watchers.clone(),
    }
  }
}

impl DatabaseTraceComponentController {
  pub fn end_tracing(self) {
    self.event.off(self.event_remover);
  }
}
