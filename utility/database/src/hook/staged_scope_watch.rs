use std::ops::DerefMut;

use parking_lot::lock_api::RawRwLock;

use crate::*;

pub fn use_db_scope<Cx: HooksCxLike>(cx: &mut Cx, scope: impl FnOnce(&mut Cx, &mut EntityScope)) {
  let (cx, db_scope) = cx.use_plain_state(EntityScope::default);

  let mut scope_watcher_drops = global_database()
    .tables
    .read()
    .iter()
    .map(|(e_id, table)| {
      let set = db_scope.entities.entry(*e_id).or_default().clone();
      let token = table.internal.entity_watchers.on(move |change| unsafe {
        match change {
          ScopedMessage::Start => {
            set.raw().lock_exclusive();
          }
          ScopedMessage::End => {
            set.raw().unlock_exclusive();
          }
          ScopedMessage::ReserveSpace(_size) => {
            // todo, optimize
          }
          ScopedMessage::Message(change) => {
            let set = &mut *set.data_ptr() as &mut EntityScopeSingle;
            match change.change {
              ValueChange::Delta(_, _) => {
                set.insert(change.idx);
              }
              ValueChange::Remove(_) => {
                set.remove(change.idx);
              }
            }
          }
        }
        false
      });
      (*e_id, token)
    })
    .collect::<FastHashMap<_, _>>();

  // todo, add assertion for no new entity type defined when in scope
  // todo, add assertion for entity should not reference any other entity out side of self scope
  scope(cx, db_scope);

  global_database()
    .tables
    .read()
    .iter()
    .for_each(|(k, table)| {
      table
        .internal
        .entity_watchers
        .off(scope_watcher_drops.remove(k).unwrap());
    });
}

#[derive(Default)]
pub struct EntityScope {
  pub entities: FastHashMap<EntityId, Arc<RwLock<EntityScopeSingle>>>,
}

impl EntityScope {
  pub fn flush_change(&self) -> FastHashMap<EntityId, EntityScopeStageChange> {
    self
      .entities
      .iter()
      .filter_map(|(k, v)| v.write().flush().map(|v| (*k, v)))
      .collect()
  }
}

#[derive(Default)]
pub struct EntityScopeSingle {
  pub current_set: FastHashSet<RawEntityHandle>,
  pub change: EntityScopeStageChange,
}

impl EntityScopeSingle {
  pub fn insert(&mut self, idx: RawEntityHandle) {
    self.change.insert(idx);
    self.current_set.insert(idx);
  }
  pub fn remove(&mut self, idx: RawEntityHandle) {
    self.change.remove(idx);
    self.current_set.remove(&idx);
  }
}

#[derive(Default, Debug)]
pub struct EntityScopeStageChange {
  pub new_inserts: FastHashSet<RawEntityHandle>,
  pub removed: FastHashSet<RawEntityHandle>,
}

impl EntityScopeStageChange {
  pub fn is_empty(&self) -> bool {
    self.new_inserts.is_empty() && self.removed.is_empty()
  }
  pub fn insert(&mut self, idx: RawEntityHandle) {
    self.new_inserts.insert(idx);
    self.removed.remove(&idx);
  }
  pub fn remove(&mut self, idx: RawEntityHandle) {
    self.removed.insert(idx);
    self.new_inserts.remove(&idx);
  }
}

impl EntityScopeSingle {
  pub fn flush(&mut self) -> Option<EntityScopeStageChange> {
    if self.change.is_empty() {
      return None;
    }
    std::mem::take(&mut self.change).into()
  }
}

pub fn use_db_scoped_staged_change<Cx: HooksCxLike>(
  cx: &mut Cx,
  scope: impl FnOnce(&mut Cx, &mut PersistenceAPI),
  on_staged_change_flushed: impl Fn(StagedDBScopeChange) + 'static,
) {
  let (cx, component_changes) = cx.use_plain_state(StagedDBScopeChangeMerger::default);
  let (cx, hydration_manager) = cx.use_plain_state(HydrationManager::default);

  use_db_scope(cx, |cx, db_entity_scope| {
    watch_db_components_in_scope(db_entity_scope, component_changes, |component_changes| {
      let mut check_pointer = PersistenceAPI {
        internal: component_changes,
        sets: db_entity_scope,
        cb: Box::new(on_staged_change_flushed),
        hydration_manager,
      };

      scope(cx, &mut check_pointer);
    });
  })
}

pub struct PersistenceAPI<'a> {
  internal: &'a StagedDBScopeChangeMerger,
  sets: &'a EntityScope,
  cb: Box<dyn Fn(StagedDBScopeChange)>,
  pub(crate) hydration_manager: &'a mut HydrationManager,
}

#[derive(Default)]
pub struct HydrationManager {
  pub(crate) labels: FastHashMap<String, RawEntityHandle>,
  pub(crate) label_changes: HydrationMetaInfoChange<RawEntityHandle>,
}

impl HydrationManager {
  pub fn flush(&mut self) -> HydrationMetaInfoChange<RawEntityHandle> {
    std::mem::take(&mut self.label_changes)
  }

  pub fn setup_hydration_label(&mut self, label: impl Into<String>, node: RawEntityHandle) {
    let label = label.into();
    self.label_changes.new_inserts.insert(label.clone(), node);
    self.label_changes.removed.remove(&label);
    self.labels.insert(label, node);
  }

  pub fn delete_hydration_label(&mut self, label: impl Into<String>) {
    let label = label.into();
    self.label_changes.removed.insert(label.clone());
    self.label_changes.new_inserts.remove(&label);
    self.labels.remove(&label);
  }
}

impl<'a> PersistenceAPI<'a> {
  /// this must be called outside the db mutation scope, or it will deadlock.
  pub fn notify_checkpoint(&mut self, label: &str) {
    let changes = self.internal.flush_buffered_changes();
    let changes = StagedDBScopeChange {
      label: label.to_string(),
      component_changes: changes,
      entity_changes: self.sets.flush_change(),
      hydration_changes: self.hydration_manager.flush(),
    };
    (self.cb)(changes);
  }

  pub fn get_hydration_label(&self, label: impl Into<String>) -> Option<RawEntityHandle> {
    self.hydration_manager.labels.get(&label.into()).copied()
  }

  pub fn setup_hydration_label(&mut self, label: impl Into<String>, node: RawEntityHandle) {
    self.hydration_manager.setup_hydration_label(label, node);
  }

  pub fn delete_hydration_label(&mut self, label: impl Into<String>) {
    self.hydration_manager.delete_hydration_label(label);
  }

  /// this is a special case to flush but avoid send mutation in some case
  pub(crate) fn flush_but_not_send(&self) {
    self.internal.flush_buffered_changes();
  }
}

#[derive(Debug)]
pub struct StagedDBScopeChange {
  pub label: String,
  pub component_changes: FastHashMap<ComponentId, StagedComponentChangeBuffer>,
  pub entity_changes: FastHashMap<EntityId, EntityScopeStageChange>,
  pub hydration_changes: HydrationMetaInfoChange<RawEntityHandle>,
}

impl StagedDBScopeChange {
  pub fn is_empty(&self) -> bool {
    self.component_changes.is_empty() && self.entity_changes.is_empty()
  }
}

#[derive(Default)]
pub struct StagedDBScopeChangeMerger {
  components: FastHashMap<ComponentId, StagedComponentChange>,
}

impl StagedDBScopeChangeMerger {
  pub fn flush_buffered_changes(&self) -> FastHashMap<ComponentId, StagedComponentChangeBuffer> {
    self
      .components
      .iter()
      .filter_map(|(k, v)| v.flush_changes().map(|v| (*k, v)))
      .collect()
  }
}

#[derive(Clone)]
struct StagedComponentChange {
  is_foreign_key: bool,
  changes: Arc<RwLock<StagedComponentChangeBuffer>>,
}

pub type StagedComponentChangeBuffer = FastHashMap<
  RawEntityHandle,
  ValueChange<DBFastSerializeSmallBufferOrForeignKey<RawEntityHandle>>,
>;

impl StagedComponentChange {
  unsafe fn start_change(&self) {
    self.changes.raw().lock_exclusive();
  }
  unsafe fn end_change(&self) {
    self.changes.raw().unlock_exclusive();
  }
  unsafe fn notify_change(
    &self,
    idx: RawEntityHandle,
    change: ValueChange<(DataPtr, *const dyn DataBaseDataTypeDyn)>,
  ) {
    let changes = &mut *self.changes.data_ptr() as &mut StagedComponentChangeBuffer;

    let change = if self.is_foreign_key {
      match change {
        ValueChange::Delta((_, new), old) => {
          let new = new as *const RawEntityHandle;
          let new = &*new as &RawEntityHandle;

          let old = old.map(|(old, _)| {
            let old = old as *const RawEntityHandle;
            let old = &*old as &RawEntityHandle;
            *old
          });
          ValueChange::Delta(*new, old)
        }
        ValueChange::Remove((previous, _)) => {
          let previous = previous as *const RawEntityHandle;
          let previous = &*previous as &RawEntityHandle;
          ValueChange::Remove(*previous)
        }
      }
      .map(DBFastSerializeSmallBufferOrForeignKey::ForeignKey)
    } else {
      match change {
        ValueChange::Delta((_, new), old) => {
          let new = &*new as &dyn DataBaseDataTypeDyn;
          let new = new.fast_serialize_into_buffer();
          let old = old.map(|(old, _)| {
            let old = &*old as &dyn DataBaseDataTypeDyn;
            old.fast_serialize_into_buffer()
          });
          ValueChange::Delta(new, old)
        }
        ValueChange::Remove((_, previous)) => {
          let previous = &*previous as &dyn DataBaseDataTypeDyn;
          ValueChange::Remove(previous.fast_serialize_into_buffer())
        }
      }
      .map(DBFastSerializeSmallBufferOrForeignKey::Pod)
    };

    merge_change(changes, (idx, change));
  }
  fn flush_changes(&self) -> Option<StagedComponentChangeBuffer> {
    let mut changes = self.changes.write();
    let changes = changes.deref_mut();
    if changes.is_empty() {
      None
    } else {
      std::mem::take(changes).into()
    }
  }
}

pub fn watch_db_components_in_scope(
  entity_scope: &EntityScope,
  scoped_change: &mut StagedDBScopeChangeMerger,
  inner: impl FnOnce(&mut StagedDBScopeChangeMerger),
) {
  let db = global_database();
  let tables = db.tables.read();

  let mut remove_tokens = tables
    .iter()
    .map(|(e_id, v)| {
      let components = v.internal.components.read();
      let remove_tokens = components
        .iter()
        .map(|(c_id, v)| {
          let entity_scope = entity_scope.entities.get(e_id).unwrap().clone();
          let change_collector = scoped_change
            .components
            .entry(*c_id)
            .or_insert_with(|| StagedComponentChange {
              is_foreign_key: v.as_foreign_key.is_some(),
              changes: Default::default(),
            })
            .clone();

          let remove_token = v.data_watchers.on(move |change| unsafe {
            match change {
              ScopedMessage::Start => {
                // in [EntityComponentGroup::entity_writer_dyn], we always emit
                // entities start first, so if we are using entity creator, the
                // entity scope will be locked. In other case, we do a shared lock
                if !entity_scope.is_locked_exclusive() {
                  entity_scope.raw().lock_shared()
                }
                change_collector.start_change();
              }
              // in EntityWriterUntyped drop, we always emit entities end after all
              // component writers drop. In other cast, we unlock the shared lock we locked
              ScopedMessage::End => {
                if !entity_scope.is_locked_exclusive() {
                  entity_scope.raw().unlock_shared()
                }
                change_collector.end_change();
              }
              ScopedMessage::ReserveSpace(_size) => {
                // todo, optimize
              }
              ScopedMessage::Message(change) => {
                let entity_scope = &*entity_scope.data_ptr() as &EntityScopeSingle;
                if entity_scope.current_set.contains(&change.idx) {
                  change_collector.notify_change(change.idx, change.change);
                }
              }
            }
            false
          });
          (*c_id, remove_token)
        })
        .collect::<FastHashMap<_, _>>();

      (*e_id, remove_tokens)
    })
    .collect::<FastHashMap<_, _>>();

  drop(tables);

  inner(scoped_change);

  global_database()
    .tables
    .read()
    .iter()
    .for_each(|(e_id, v)| {
      let mut removers = remove_tokens.remove(e_id).unwrap();
      v.internal.components.read().iter().for_each(|(k, v)| {
        let token = removers.remove(k).unwrap();
        v.data_watchers.off(token);
      });
    });
}
