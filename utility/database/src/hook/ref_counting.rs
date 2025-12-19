use std::future::Future;

use futures::future::join_all;

use crate::*;

type ForeignKeyChange = BoxedDynQuery<RawEntityHandle, ValueChange<RawEntityHandle>>;
pub type DBAllForeignKeyChange = FastHashMap<(EntityId, ComponentId), UseResult<ForeignKeyChange>>;

type ForeignKeyChangeFut = Pin<FrameBox<dyn Future<Output = ForeignKeyChange> + Send + Sync>>;

type RevOwnershipForeignKeysConfig = FastHashSet<ComponentId>;

pub fn use_db_all_foreign_key_change(
  cx: &mut impl DBHookCxLike,
  config: &RevOwnershipForeignKeysConfig,
) -> UseResult<DBAllForeignKeyChange> {
  let mut changes = DBAllForeignKeyChange::default();

  cx.skip_if_not_waked(|cx| {
    for (e_id, ecg) in global_database().ecg_tables.read().iter() {
      cx.skip_if_not_waked(|cx| {
        cx.keyed_scope(e_id, |cx| {
          for (c_id, ref_e_id) in ecg.inner.foreign_keys.read().iter() {
            cx.keyed_scope(c_id, |cx| {
              if !config.contains(c_id) {
                cx.scope(|cx| {
                  let change = cx
                    .use_dual_query_impl::<Option<RawEntityHandle>>(*c_id, *e_id, None)
                    .dual_query_filter_map(|v| v)
                    .map(|v| v.delta().into_boxed());

                  changes.insert((*ref_e_id, *c_id), change);
                });
              } else {
                cx.scope(|cx| {
                  let relation = cx.use_db_rev_ref_tri_view_impl(*c_id, *e_id);
                  let change = cx
                    .use_dual_query_set_raw(*ref_e_id)
                    .fanout(relation, cx)
                    .map(|v| v.delta().delta_key_as_value().into_boxed());

                  changes.insert((*e_id, *c_id), change);
                });
              }
            });
          }
        });
      });
    }
  });

  if cx.is_spawning_stage() {
    UseResult::SpawnStageReady(changes)
  } else {
    UseResult::NotInStage
  }
}

type RefCountChange = FastHashMap<RawEntityHandle, ValueChange<u32>>;
pub type DBAllEntityRefCountChange = FastHashMap<EntityId, RefCountChange>;

pub const DEBUG: bool = false;

pub fn use_db_all_entity_ref_count_change(
  cx: &mut impl DBHookCxLike,
  config: &RevOwnershipForeignKeysConfig,
) -> UseResult<DBAllEntityRefCountChange> {
  let fk_change = use_db_all_foreign_key_change(cx, config);

  let (cx, rc) = cx.use_plain_state(DBRefCounting::default);

  if let QueryHookStage::SpawnTask { spawner, .. } = cx.stage() {
    let changes = fk_change.expect_spawn_stage_ready();

    let mut update_groups = FastHashMap::<EntityId, Vec<ForeignKeyChangeFut>>::default();
    for ((refed_e_id, _), change) in changes {
      let updates = update_groups.entry(refed_e_id).or_default();
      updates.push(change.expect_spawn_stage_future())
    }

    let mut all_entity_updates = Vec::default();
    for (e_id, updates) in update_groups {
      let counts = rc.ref_counts.entry(e_id).or_default().clone();
      let spawner = spawner.clone();

      let fut = async move {
        let updates = join_all(updates).await;

        spawner
          .spawn_task(move || {
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
              let name = names.entities.get(&e_id).unwrap();

              println!(
                "ref count changes for entity {}: {:#?}",
                name, ref_count_changes
              );
            }

            (e_id, ref_count_changes)
          })
          .await
      };
      all_entity_updates.push(fut);
    }

    use futures::FutureExt;
    let all_entity_updates = join_all(all_entity_updates)
      .map(|updates| updates.into_iter().collect::<FastHashMap<_, _>>());

    UseResult::SpawnStageFuture(pin_box_in_frame(all_entity_updates))
  } else {
    UseResult::NotInStage
  }
}

#[derive(Default)]
struct DBRefCounting {
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
