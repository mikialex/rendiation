use crate::*;

pub fn use_db_scoped_staged_change<Cx: DBHookCxLike>(
  cx: &mut Cx,
  scope: impl FnOnce(&mut Cx, &CheckPointCreator),
  on_staged_change_flushed: impl Fn(StagedDBScopeChange) + 'static,
) {
  let (cx, change_merger) = cx.use_plain_state(StagedDBScopeChangeMerger::default);

  let check_pointer = change_merger.create_check_point_creator(on_staged_change_flushed);

  use_db_scope(
    cx,
    |e_id, set_change| {},
    |cx, db_entity_scope| {
      watch_db_components_in_scope(
        cx,
        |cx| {
          scope(cx, &check_pointer);
        },
        db_entity_scope,
        change_merger,
      );
    },
  )
}

pub fn use_db_incremental_persistence<Cx: DBHookCxLike>(
  cx: &mut Cx,
  scope: impl FnOnce(&mut Cx, &CheckPointCreator),
) {
  use_db_scoped_staged_change(
    cx,
    |cx, cp| {
      // reload data if last time crashed

      //
      scope(cx, cp)
    },
    |change| {
      // write to disk
    },
  )
  //
}

// pub struct UndoRedoController {
//   internal: CheckPointCreator,
// }

// pub fn use_db_undo_redo<Cx: DBHookCxLike>(
//   cx: &mut Cx,
//   inner: impl FnOnce(&mut Cx, &mut UndoRedoController),
// ) {
//   use_db_scoped_staged_change(
//     cx,
//     |cx, check_pointer| inner(cx, todo!()),
//     |change| {
//       //
//     },
//   )
// }

pub fn use_debug_tracing<Cx: DBHookCxLike>(cx: &mut Cx, inner: impl FnOnce(&mut Cx)) {
  let (cx, change_merger) = cx.use_plain_state(StagedDBScopeChangeMerger::default);

  use_db_scope(
    cx,
    |e_id, set_change| {},
    |cx, scope| {
      watch_db_components_in_scope(cx, inner, scope, change_merger);
    },
  );
}

pub struct CheckPointCreator {
  internal: StagedDBScopeChangeMerger,
  cb: Box<dyn Fn(StagedDBScopeChange)>,
}

impl CheckPointCreator {
  /// this must called outside of the db mutation scope, or it will deadlock.
  pub fn notify_checkpoint(&self, _label: &str) {
    let changes = self.internal.flush_buffered_changes();
    (self.cb)(changes);
  }
}

pub struct StagedDBScopeChange {
  pub component_changes: FastHashMap<ComponentId, Vec<u8>>,
  pub entity_changes: FastHashMap<EntityId, StagedEntityChange>,
}

#[derive(Default, Clone)]
pub struct StagedDBScopeChangeMerger {
  internal: Arc<RwLock<StagedDBScopeChangeMergerInternal>>,
}

impl StagedDBScopeChangeMerger {
  pub fn create_check_point_creator(
    &self,
    cb: impl Fn(StagedDBScopeChange) + 'static,
  ) -> CheckPointCreator {
    CheckPointCreator {
      internal: self.clone(),
      cb: Box::new(cb),
    }
  }
}

#[derive(Default)]
pub struct StagedDBScopeChangeMergerInternal {
  components: FastHashMap<ComponentId, Box<dyn ChangeStaging>>,
  entities: FastHashMap<EntityId, StagedEntityChange>,
}

impl StagedDBScopeChangeMerger {
  pub fn flush_buffered_changes(&self) -> StagedDBScopeChange {
    let mut internal = self.internal.write();
    StagedDBScopeChange {
      component_changes: internal
        .components
        .iter()
        .map(|(k, v)| (*k, v.flush_changes()))
        .collect(),
      entity_changes: std::mem::take(&mut internal.entities),
    }
  }
}

pub struct StagedEntityChange {
  pub new_inserted_entities: FastHashSet<RawEntityHandle>,
  pub deleted_entities: FastHashSet<RawEntityHandle>,
}

trait ChangeStaging: Send + Sync + 'static {
  unsafe fn start_change(&self);
  unsafe fn end_change(&self);
  unsafe fn merge_change(
    &self,
    idx: RawEntityHandle,
    change: ValueChange<(DataPtr, *const dyn DataBaseDataTypeDyn)>,
  );
  fn flush_changes(&self) -> Vec<u8>;
  fn clone_boxed(&self) -> Box<dyn ChangeStaging>;
}

#[derive(Clone)]
struct StagedComponentChange<V> {
  changes: Arc<RwLock<FastHashMap<RawEntityHandle, ValueChange<V>>>>,
}

impl<V: CValue> ChangeStaging for StagedComponentChange<V> {
  unsafe fn start_change(&self) {}
  unsafe fn end_change(&self) {}
  unsafe fn merge_change(
    &self,
    idx: RawEntityHandle,
    change: ValueChange<(DataPtr, *const dyn DataBaseDataTypeDyn)>,
  ) {
    todo!()
  }
  fn flush_changes(&self) -> Vec<u8> {
    todo!()
  }
  fn clone_boxed(&self) -> Box<dyn ChangeStaging> {
    Box::new(self.clone())
  }
}

pub fn watch_db_components_in_scope<Cx: DBHookCxLike>(
  cx: &mut Cx,
  inner: impl FnOnce(&mut Cx),
  entity_scope: &EntityScope,
  scoped_change: &StagedDBScopeChangeMerger,
) {
  let db = global_database();
  let tables = db.ecg_tables.read();
  let scoped_change = scoped_change.internal.write();

  let mut remove_tokens = tables
    .iter()
    .map(|(e_id, v)| {
      let components = v.inner.components.read();
      let remove_tokens = components
        .iter()
        .map(|(c_id, v)| {
          use parking_lot::lock_api::RawRwLock;
          let entity_scope = entity_scope.entities.get(e_id).unwrap().clone();
          let change_collector = scoped_change.components.get(c_id).unwrap().clone_boxed();

          let remove_token = v.data_watchers.on(move |change| unsafe {
            match change {
              ScopedMessage::Start => {
                // in [EntityComponentGroup::entity_writer_dyn], we always emit
                // entities start first, so if we are using entity creator, the
                // entity scope will be locked. In other case, we do a shared lock
                // the entity scope will always be read and write by one thread
                // todo, debug validation
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
              ScopedMessage::Message(change) => {
                let entity_scope = &*entity_scope.data_ptr() as &FastHashSet<RawEntityHandle>;
                if entity_scope.contains(&change.idx) {
                  change_collector.merge_change(change.idx, change.change);
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

  drop(scoped_change);
  drop(tables);

  inner(cx);

  global_database()
    .ecg_tables
    .read()
    .iter()
    .for_each(|(e_id, v)| {
      let mut removers = remove_tokens.remove(e_id).unwrap();
      v.inner.components.read().iter().for_each(|(k, v)| {
        let token = removers.remove(k).unwrap();
        v.data_watchers.off(token);
      });
    });
}
