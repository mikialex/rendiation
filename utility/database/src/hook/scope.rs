use crate::*;

pub fn use_db_scope<Cx: DBHookCxLike>(
  cx: &mut Cx,
  on_change: impl Fn(EntityId, &ChangePtr) + Send + Sync + Clone + 'static,
  scope: impl FnOnce(&mut Cx, &mut EntityScope),
) {
  let (cx, db_scope) = cx.use_plain_state(EntityScope::default);

  let mut scope_watcher_drops = global_database()
    .ecg_tables
    .read()
    .iter()
    .map(|(k, table)| {
      use parking_lot::lock_api::RawRwLock;
      let set = db_scope.entities.entry(*k).or_default().clone();
      let on_change = on_change.clone();
      let e_id = *k;
      let token = table.inner.entity_watchers.on(move |change| unsafe {
        on_change(e_id, change);
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
    .collect::<FastHashMap<_, _>>();

  // todo, add assertion for no new entity type defined when in scope
  // todo, add assertion for entity should not reference any other entity out side of self scope
  scope(cx, db_scope);

  global_database()
    .ecg_tables
    .read()
    .iter()
    .for_each(|(k, table)| {
      table
        .inner
        .entity_watchers
        .off(scope_watcher_drops.remove(k).unwrap());
    });
}

#[derive(Default)]
pub struct EntityScope {
  pub entities: FastHashMap<EntityId, Arc<RwLock<FastHashSet<RawEntityHandle>>>>,
}
