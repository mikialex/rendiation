use crate::*;

pub fn use_db_scope<Cx: DBHookCxLike>(cx: &mut Cx, scope: impl FnOnce(&mut Cx)) {
  let (cx, db_scope) = cx.use_plain_state(|| EntityScope::new(global_database()));
  db_scope.enter();
  scope(cx);
  db_scope.exit();
}

struct EntityScope {
  db: Database,
  scope_watcher_drops: Option<FastHashMap<EntityId, RemoveToken<ChangePtr>>>,
  entities: FastHashMap<EntityId, Arc<RwLock<FastHashSet<RawEntityHandle>>>>,
}

impl EntityScope {
  pub fn new(db: Database) -> Self {
    Self {
      db,
      scope_watcher_drops: Default::default(),
      entities: Default::default(),
    }
  }

  // todo, add assertion for no new entity type defined when in scope
  // todo, add assertion for entity should not reference any other entity out side of self scope
  pub fn enter(&mut self) {
    assert!(self.scope_watcher_drops.is_none());
    let scope_watcher_drops = self
      .db
      .ecg_tables
      .read()
      .iter()
      .map(|(k, table)| {
        use parking_lot::lock_api::RawRwLock;
        let set = self.entities.entry(*k).or_default().clone();
        let token = table.inner.entity_watchers.on(move |change| unsafe {
          match change {
            ScopedMessage::Start => {
              set.raw().lock_exclusive();
            }
            ScopedMessage::End => {
              set.raw().lock_exclusive();
            }
            ScopedMessage::Message(change) => {
              let set = &mut *set.data_ptr() as &mut FastHashSet<RawEntityHandle>;
              match change.change {
                ValueChange::Delta(_, _) => {
                  assert!(set.insert(change.idx));
                }
                ValueChange::Remove(_) => {
                  assert!(set.remove(&change.idx));
                }
              }
            }
          }
          false
        });
        (*k, token)
      })
      .collect();

    self.scope_watcher_drops = Some(scope_watcher_drops);
  }

  pub fn exit(&mut self) {
    let mut scope_watcher_drops = self.scope_watcher_drops.take().unwrap();
    self.db.ecg_tables.read().iter().for_each(|(k, table)| {
      table
        .inner
        .entity_watchers
        .off(scope_watcher_drops.remove(k).unwrap());
    });
  }
}
