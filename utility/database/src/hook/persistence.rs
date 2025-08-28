use std::fs::File;
use std::io::Write;

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
        if persist_cx.is_new_crated {
          init_for_new_persistent_scope();
          persist_cx.is_new_crated = true;
        } else {
          // todo, load from file
        }
      }

      scope(cx, cp)
    },
    move |change| {
      sender.unbounded_send(change).ok();
    },
  )
}

struct PersistentContext {
  is_new_crated: bool,
  change_sender: UnboundedSender<StagedDBScopeChange>,
}

impl Default for PersistentContext {
  fn default() -> Self {
    let assume_last_run_file_path = std::env::current_dir().unwrap().join("db_save.bin");

    let is_new_crated = !assume_last_run_file_path.exists();

    let base_id = if is_new_crated { 0 } else { todo!() };

    let mut file = File::create(assume_last_run_file_path).unwrap();

    let (change_sender, mut receiver) = futures::channel::mpsc::unbounded::<StagedDBScopeChange>();

    let mut writer = PersistIdMapperForWrite {
      base_id,
      mapping: Default::default(),
      db_name_mapping: global_database().name_mapping.clone(),
    };

    // we should get a thread pool?
    // this thread is detached, but it's ok
    std::thread::spawn(move || {
      while let Some(change) = futures::executor::block_on(receiver.next()) {
        if change.is_empty() {
          continue;
        }
        println!("write {:?}", change);

        let change_to_write = writer.map(change);
        change_to_write.write(&mut file).unwrap();
      }
      file.flush().unwrap();
    });

    Self {
      is_new_crated,
      change_sender,
    }
  }
}

type PersistEntityId = u64;

type PersistEntityTypeId = String;
type PersistComponentTypeId = String;

struct PersistIdMapperForWrite {
  base_id: PersistEntityId,
  mapping: FastHashMap<RawEntityHandle, PersistEntityId>,
  db_name_mapping: Arc<RwLock<DBNameMapping>>,
}

impl PersistIdMapperForWrite {
  pub fn map(&mut self, change: StagedDBScopeChange) -> PersistStagedDBScopeChange {
    let name_mapping = self.db_name_mapping.read();

    // create all new created entities first, for latter mapping
    let new_entities = change
      .entity_changes
      .iter()
      .map(|(k, v)| {
        let k = name_mapping.entities.get(k).unwrap().clone();
        let v = v
          .new_inserts
          .iter()
          .map(|k| {
            self.base_id += 1;
            let new_id = self.base_id;
            self.mapping.insert(*k, new_id);
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
          .map(|k| self.mapping.remove(k).unwrap())
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
  pub component_changes: FastHashMap<PersistEntityTypeId, PersistStagedComponentChangeBuffer>,
  pub entity_changes: FastHashMap<PersistComponentTypeId, PersistEntityScopeStageChange>,
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
}
