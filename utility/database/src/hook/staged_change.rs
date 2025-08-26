use crate::*;

#[derive(Default)]
pub struct CheckPointCreator {}

impl CheckPointCreator {
  pub fn notify_checkpoint(&self, _label: &str) {
    //
  }
}

pub fn use_db_scoped_staged_change<Cx: DBHookCxLike>(
  cx: &mut Cx,
  scope: impl FnOnce(&mut Cx, &mut CheckPointCreator),
  on_staged_change_flushed: impl FnMut(StagedDBScopeChange),
) {
  use_db_scope(
    cx,
    |set_change| {},
    |cx, db_scope| {
      // todo setup change watchers
      // global_database().ecg_tables

      let mut notifier = Default::default();

      scope(cx, &mut notifier);

      // todo detach change watchers
    },
  )
}

pub fn use_db_incremental_persistence<Cx: DBHookCxLike>(
  cx: &mut Cx,
  scope: impl FnOnce(&mut Cx, &mut CheckPointCreator),
) {
  use_db_scoped_staged_change(cx, scope, |change| {
    // reload data if last time crashed

    //
  })
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
  use_db_scope(
    cx,
    |set_change| {},
    |cx, scope| {
      watch_db_components_in_scope(cx, inner, scope);
    },
  );
}

pub struct StagedDBScopeChange {
  internal: FastHashMap<RawEntityHandle, Box<dyn ChangeMerger>>,
}

trait ChangeMerger {
  fn merge_change(
    &mut self,
    idx: RawEntityHandle,
    change: (DataPtr, *const dyn DataBaseDataTypeDyn),
  ) -> bool;
}

struct ComponentChangeMerger<V> {
  changes: FastHashMap<RawEntityHandle, ValueChange<V>>,
}

impl<V> ChangeMerger for ComponentChangeMerger<V> {
  fn merge_change(
    &mut self,
    idx: RawEntityHandle,
    change: (DataPtr, *const dyn DataBaseDataTypeDyn),
  ) -> bool {
    todo!()
  }
}

pub fn watch_db_components_in_scope<Cx: DBHookCxLike>(
  cx: &mut Cx,
  inner: impl FnOnce(&mut Cx),
  entity_scope: &EntityScope,
) {
  let db = global_database();
  let tables = db.ecg_tables.read();
  let mut remove_tokens = tables
    .iter()
    .map(|(e_id, v)| {
      let components = v.inner.components.read();
      let remove_tokens = components
        .iter()
        .map(|(c_id, v)| {
          let entity_scope = entity_scope.entities.get(e_id).unwrap();

          let remove_token = v.data_watchers.on(move |change| {
            match change {
              ScopedMessage::Start => todo!(),
              ScopedMessage::End => todo!(),
              ScopedMessage::Message(_) => todo!(),
            }
            false
          });
          (*c_id, remove_token)
        })
        .collect::<FastHashMap<_, _>>();

      (*e_id, remove_tokens)
    })
    .collect::<FastHashMap<_, _>>();

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
