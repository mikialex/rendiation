use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

use futures::channel::mpsc::UnboundedSender;
use futures::StreamExt;

use crate::*;

/// all db mutation in this scope will be automatically and incrementally saved.
/// when init, if the previous saved file is found, it will be loaded or the
/// `init_for_new_persistent_scope` will be called.
pub fn use_persistent_db_scope<Cx: HooksCxLike>(
  cx: &mut Cx,
  init_for_new_persistent_scope: impl FnOnce(),
  scope: impl FnOnce(&mut Cx, &CheckPointCreator),
) {
  let (cx, persist_cx) = cx.use_plain_state(PersistentContext::default);
  let sender = persist_cx.change_sender.clone();

  use_db_scoped_staged_change(
    cx,
    |cx, cp| {
      if cx.is_creating() {
        if let Some(init_data) = persist_cx.init_from_file.take() {
          init_data.write_into_db();
          // avoid send this init change to data persist worker, because this data
          // has already been persisted
          cp.flush_but_not_send();
        } else {
          init_for_new_persistent_scope();
        }
      }

      scope(cx, cp)
    },
    move |change| {
      sender.unbounded_send(change).ok();
    },
  )
}

struct DBPersistReadBackInitWrite {
  data_to_write: Vec<PersistStagedDBScopeChange>,
  writer: Arc<RwLock<PersistIdMapper>>,
}

impl DBPersistReadBackInitWrite {
  pub fn write_into_db(self) {
    let mut writer = self.writer.write();
    for change in self.data_to_write {
      writer.write_db(change);
    }
  }
}

struct PersistentContext {
  init_from_file: Option<DBPersistReadBackInitWrite>,
  change_sender: UnboundedSender<StagedDBScopeChange>,
}

impl Default for PersistentContext {
  fn default() -> Self {
    let assume_last_run_file_path = std::env::current_dir().unwrap().join("db_save.bin");

    let is_new_created = !assume_last_run_file_path.exists();

    let mut file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(false)
      .open(assume_last_run_file_path)
      .unwrap(); // todo buffer read write

    // assure read from start
    file.seek(SeekFrom::Start(0)).unwrap();

    // todo, do it in async task
    let mut last_success_read_offset = file.stream_position().unwrap();
    let mut init_data = Vec::new();
    if !is_new_created {
      if let Some(previous_transaction) = PersistStagedDBScopeChange::read(&mut file) {
        init_data.push(previous_transaction);
        last_success_read_offset = file.stream_position().unwrap();
      }
    }
    file.set_len(last_success_read_offset).unwrap();
    file
      .seek(SeekFrom::Start(last_success_read_offset))
      .unwrap();

    let (change_sender, mut receiver) = futures::channel::mpsc::unbounded::<StagedDBScopeChange>();

    let writer = PersistIdMapper {
      base_id: 0,
      mapping: Default::default(),
      rev_mapping: Default::default(),
      db_name_mapping: global_database().name_mapping.clone(),
    };

    let writer = Arc::new(RwLock::new(writer));
    let writer_ = writer.clone();

    // we should get a thread pool?
    // this thread is detached, but it's ok
    std::thread::spawn(move || {
      while let Some(change) = futures::executor::block_on(receiver.next()) {
        if change.is_empty() {
          continue;
        }
        println!("write {:?}", change);
        let mut writer = writer.write();

        let change_to_write = writer.map(change);
        change_to_write.write(&mut file).unwrap();
      }
      file.flush().unwrap();
    });

    Self {
      init_from_file: if is_new_created {
        None
      } else {
        Some(DBPersistReadBackInitWrite {
          data_to_write: init_data,
          writer: writer_,
        })
      },
      change_sender,
    }
  }
}

type PersistEntityId = u64;

struct PersistIdMapper {
  base_id: PersistEntityId,
  mapping: FastHashMap<RawEntityHandle, PersistEntityId>,
  rev_mapping: FastHashMap<PersistEntityId, RawEntityHandle>,
  db_name_mapping: Arc<RwLock<DBNameMapping>>,
}

impl PersistIdMapper {
  pub fn write_db(&mut self, change: PersistStagedDBScopeChange) {
    let name_mapping = self.db_name_mapping.read();
    let db = global_database();

    // create all new created entities first, for later mapping
    let tables = db.ecg_tables.read();
    for (entity_name, v) in &change.entity_changes {
      let e_id = name_mapping.entities_inv.get(entity_name).unwrap();
      let entity_group = tables.get(e_id).unwrap();
      let mut writer = entity_group.entity_writer_dyn();
      for entity_p_id in &v.new_inserts {
        let new_id = writer.new_entity();
        self.rev_mapping.insert(*entity_p_id, new_id);
        self.mapping.insert(new_id, *entity_p_id);
      }
    }

    for (com_name, changes) in change.component_changes {
      let com_id = name_mapping.components_inv.get(&com_name).unwrap();
      let e_id = name_mapping.component_to_entity.get(com_id).unwrap();
      let entity_group = tables.get(e_id).unwrap();
      entity_group.access_component(*com_id, |component| {
        let mut com_writer = component.write_untyped();
        for (entity_p_id, change) in changes {
          let target_entity_handle = self.rev_mapping.get(&entity_p_id).unwrap();

          let change = change.map(|fk_p_id| *self.rev_mapping.get(&fk_p_id).unwrap());

          unsafe {
            com_writer.write_by_small_serialize_data(*target_entity_handle, change);
          }
        }
      });
    }

    // // remove all new removed entities here.
    for (entity_name, v) in &change.entity_changes {
      let e_id = name_mapping.entities_inv.get(entity_name).unwrap();
      let entity_group = tables.get(e_id).unwrap();
      let mut writer = entity_group.entity_writer_dyn();
      for entity_p_id in &v.removed {
        let db_entity_id = self.rev_mapping.remove(entity_p_id).unwrap();
        self.mapping.remove(&db_entity_id).unwrap();
        writer.delete_entity(db_entity_id);
      }
    }
  }

  pub fn map(&mut self, change: StagedDBScopeChange) -> PersistStagedDBScopeChange {
    let name_mapping = self.db_name_mapping.read();

    // create all new created entities first, for later mapping
    let new_entities = change
      .entity_changes
      .iter()
      .map(|(k, v)| {
        let k = name_mapping.entities.get(k).unwrap().clone();
        let v = v
          .new_inserts
          .iter()
          .map(|db_entity_id| {
            self.base_id += 1;
            let new_id = self.base_id;
            self.mapping.insert(*db_entity_id, new_id);
            self.rev_mapping.insert(new_id, *db_entity_id);
            new_id
          })
          .collect::<FastHashSet<_>>();
        (k, v)
      })
      .collect::<FastHashMap<_, _>>();

    let component_changes = change
      .component_changes
      .into_iter()
      .map(|(k, v)| {
        let k = name_mapping.components.get(&k).unwrap().clone();
        let v = v
          .into_iter()
          .filter_map(|(k, v)| {
            let k = self.mapping.get(&k).unwrap();
            v.into_new_value().map(|v| {
              let v = v.map(|v| *self.mapping.get(&v).unwrap());
              (*k, v)
            })
          })
          .collect();
        (k, v)
      })
      .collect();

    // remove all new removed entities here.
    let mut remove_entities = change
      .entity_changes
      .iter()
      .map(|(k, v)| {
        let k = name_mapping.entities.get(k).unwrap().clone();
        let v = v
          .removed
          .iter()
          .map(|k| {
            let entity_p_id = self.mapping.remove(k).unwrap();
            self.rev_mapping.remove(&entity_p_id).unwrap();
            entity_p_id
          })
          .collect::<FastHashSet<_>>();
        (k, v)
      })
      .collect::<FastHashMap<_, _>>();

    let entity_changes = new_entities
      .into_iter()
      .map(|(k, new_inserts)| {
        let removed = remove_entities.remove(&k).unwrap();
        let v = PersistEntityScopeStageChange {
          new_inserts,
          removed,
        };
        (k, v)
      })
      .collect();

    PersistStagedDBScopeChange {
      component_changes,
      entity_changes,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistStagedDBScopeChange {
  pub component_changes: FastHashMap<String, PersistStagedComponentChangeBuffer>,
  pub entity_changes: FastHashMap<String, PersistEntityScopeStageChange>,
}

// only contains setting
pub type PersistStagedComponentChangeBuffer =
  FastHashMap<PersistEntityId, DBFastSerializeSmallBufferOrForeignKey<PersistEntityId>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistEntityScopeStageChange {
  pub new_inserts: FastHashSet<PersistEntityId>,
  pub removed: FastHashSet<PersistEntityId>,
}

impl PersistStagedDBScopeChange {
  pub fn write(&self, target: &mut impl Write) -> Option<()> {
    rmp_serde::encode::write(target, self).ok()
  }

  pub fn read(source: &mut impl Read) -> Option<Self> {
    rmp_serde::decode::from_read(source).ok()
  }
}
