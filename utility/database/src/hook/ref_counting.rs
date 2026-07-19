use futures::{future::join_all, FutureExt};

use crate::*;

type ForeignKeyChangeImpl = BoxedDynQuery<RawEntityHandle, ValueChange<RawEntityHandle>>;

pub struct ForeignKeyChange {
  pub refed_entity_id: EntityId,
  pub source_entity_id: EntityId,
  pub component_id: ComponentId,
  pub change: ForeignKeyChangeImpl,
}

pub type DBAllForeignKeyChange = Arc<Vec<ForeignKeyChange>>;

type RevOwnershipForeignKeysConfig = FastHashSet<ComponentId>;

struct ForeignKeyChangeCollecting {
  refed_entity_id: EntityId,
  source_entity_id: EntityId,
  component_id: ComponentId,
  change: UseResult<ForeignKeyChangeImpl>,
}

pub fn use_db_all_foreign_key_change(
  cx: &mut impl DBHookCxLike,
  config: &RevOwnershipForeignKeysConfig,
) -> UseResult<DBAllForeignKeyChange> {
  let mut changes_collector = Vec::default();

  cx.skip_if_not_waked(|cx| {
    for (e_id, table) in global_database().tables.read().iter() {
      cx.skip_if_not_waked(|cx| {
        cx.keyed_scope(e_id, |cx| {
          for (c_id, ref_e_id) in table.internal.foreign_keys.read().iter() {
            cx.keyed_scope(c_id, |cx| {
              if !config.contains(c_id) {
                cx.scope(|cx| {
                  let change = cx
                    .use_dual_query_impl::<Option<RawEntityHandle>>(*c_id, *e_id, None)
                    .dual_query_filter_map(|v| v)
                    .map(|v| v.delta().into_boxed());

                  changes_collector.push(ForeignKeyChangeCollecting {
                    refed_entity_id: *ref_e_id,
                    source_entity_id: *e_id,
                    component_id: *c_id,
                    change,
                  });
                });
              } else {
                cx.scope(|cx| {
                  let relation = cx.use_db_rev_ref_tri_view_impl(*c_id, *e_id);
                  let change = cx
                    .use_dual_query_set_raw(*ref_e_id)
                    .fanout(relation, cx)
                    .map(|v| v.delta().delta_key_as_value().into_boxed());

                  changes_collector.push(ForeignKeyChangeCollecting {
                    refed_entity_id: *e_id,
                    source_entity_id: *ref_e_id,
                    component_id: *c_id,
                    change,
                  })
                });
              }
            });
          }
        });
      });
    }
  });

  if cx.is_spawning_stage() {
    let mut changes_to_waits = Vec::default();
    for c in changes_collector {
      let refed_entity_id = c.refed_entity_id;
      let source_entity_id = c.source_entity_id;
      let component_id = c.component_id;
      if let Some(change) = c.change.into_spawn_stage_future() {
        changes_to_waits.push(change.map(move |change| ForeignKeyChange {
          refed_entity_id,
          source_entity_id,
          component_id,
          change,
        }));
      }
    }
    UseResult::SpawnStageFuture(pin_box_in_frame(
      join_all(changes_to_waits).map(|v| Arc::new(v)),
    ))
  } else {
    UseResult::NotInStage
  }
}

type RefCountChange = FastHashMap<RawEntityHandle, ValueChange<u32>>;
pub type DBAllEntityRefCountChange = FastHashMap<EntityId, RefCountChange>;

pub const DEBUG: bool = false;

pub fn use_db_all_entity_ref_count_change(
  cx: &mut impl DBHookCxLike,
  fk_change: UseResult<DBAllForeignKeyChange>,
) -> UseResult<(DBAllEntityRefCountChange, DBRefCountingShared)> {
  let (cx, rc) = cx.use_plain_state_default_cloned::<DBRefCountingShared>();

  fk_change.map_spawn_stage_in_thread(
    cx,
    |_| true,
    |fk_changes| {
      let mut update_groups = FastHashMap::<EntityId, Vec<ForeignKeyChangeImpl>>::default();
      for c in fk_changes.iter() {
        let updates = update_groups.entry(c.refed_entity_id).or_default();
        updates.push(c.change.clone());
      }

      let mut all_entity_updates = FastHashMap::default();
      // todo, consider using rayon-like thread pool here
      for (refed_e_id, updates) in update_groups {
        let mut rc_ = rc.write();
        let counts = rc_.ref_counts.entry(refed_e_id).or_default().clone();
        drop(rc_);

        let mut counts = counts.write();
        let mut ref_count_changes = FastHashMap::default();
        let mut collector = QueryMutationCollector {
          delta: &mut ref_count_changes,
          target: &mut *counts,
        };
        for update in updates {
          update_ref_count(&mut collector, update)
        }

        if DEBUG && !ref_count_changes.is_empty() {
          let names = global_database().name_mapping.clone();
          let names = names.read();
          let name = names.entities.get(&refed_e_id).unwrap();

          println!(
            "ref count changes for entity {}: {:#?}",
            name, ref_count_changes
          );
        }
        all_entity_updates.insert(refed_e_id, ref_count_changes);
      }

      (all_entity_updates, rc)
    },
  )
}

pub type DBRefCountingShared = Arc<RwLock<DBRefCounting>>;

#[derive(Default)]
pub struct DBRefCounting {
  pub ref_counts: FastHashMap<EntityId, Arc<RwLock<RefCountingMap>>>,
}

type RefCountingMap = FastHashMap<RawEntityHandle, u32>;

fn update_ref_count(
  collector: &mut impl QueryLikeMutateTarget<RawEntityHandle, u32>,
  change: impl Query<Key = RawEntityHandle, Value = ValueChange<RawEntityHandle>>,
) {
  for (_, change) in change.iter_key_value() {
    if let Some(v) = change.old_value() {
      collector.mutate(*v, &|rc| {
        assert!(*rc >= 1);
        *rc -= 1;
      });

      if collector.get_current(*v) == Some(&0) {
        collector.remove(*v);
      }
    }

    if let Some(v) = change.new_value() {
      if collector.get_current(*v).is_none() {
        collector.set_value(*v, 1);
      } else {
        collector.mutate(*v, &|rc| {
          *rc += 1;
        });
      }
    }
  }
}
