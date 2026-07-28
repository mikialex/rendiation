use futures::{future::join_all, FutureExt};

use crate::*;

pub fn use_database_reference_integrity_checker(cx: &mut impl DBHookCxLike) {
  let (cx, _) = cx.use_plain_state(|| {
    log::warn!("reference integrity checker is enabled");
  });

  let fk_change = use_db_all_foreign_key_change(cx, &FastHashSet::default());

  let (fk_change, fk_change2) = fk_change.fork();

  let all_entity_deletions = use_db_all_entity_deletions(cx);

  let db_ref_counts = use_db_all_entity_ref_count_change(cx, fk_change2);

  let check_not_delete_still_referenced_entity = db_ref_counts
    .join(all_entity_deletions)
    .map_spawn_stage_in_thread(
      cx,
      |_| true,
      |((_, db_ref_counts), entity_deletions)| {
        // todo, consider using rayon-like thread pool here
        let db_ref_counts = db_ref_counts.read();
        for c in entity_deletions.iter() {
          if let Some(rc) = db_ref_counts.ref_counts.get(&c.entity_id){
            let rc = rc.read();
            for (entity_handle, change) in c.change.iter_key_value() {
              if change.is_removed(){
                if rc.contains_key(&entity_handle) {
                  panic!("reference integrity checker failed: entity {entity_handle} is deleted but still referenced");
                }
              }
            }
          }  // none is possible if no foreign key refed this kind of entity
        }
      },
    );

  let check_not_set_referenced_to_none_exist_entity = fk_change.map_spawn_stage_in_thread(cx, |_|{
    true
  }, |fk_changes|{
      // todo, consider using rayon-like thread pool here
    for c in fk_changes.iter() {
      let ref_entity_set_reader = get_db_set_view_dyn(c.refed_entity_id);
      for (item_handle, ref_handle_change) in c.change.iter_key_value() {
        if let Some(new_ref) = ref_handle_change.new_value() {
          if !ref_entity_set_reader.contains(new_ref) {
            panic!(
              "reference integrity checker failed: entity {item_handle} ref to a entity {new_ref} that not exist"
            );
          }
        }
      }
    }
  });

  let _ = check_not_delete_still_referenced_entity.use_assure_result(cx);
  let _ = check_not_set_referenced_to_none_exist_entity.use_assure_result(cx);
}

type EntityDeletionChangeImpl = BoxedDynQuery<RawEntityHandle, ValueChange<()>>;
type DBAllEntityDeletionChange = Arc<Vec<EntityDeletionChange>>;

struct EntityDeletionChange {
  entity_id: EntityId,
  change: EntityDeletionChangeImpl,
}

struct EntityDeletionChangeCollecting {
  entity_id: EntityId,
  change: UseResult<EntityDeletionChangeImpl>,
}

fn use_db_all_entity_deletions(cx: &mut impl DBHookCxLike) -> UseResult<DBAllEntityDeletionChange> {
  let mut changes_collector = Vec::default();

  cx.skip_if_not_waked(|cx| {
    for (e_id, _) in global_database().tables.read().iter() {
      cx.skip_if_not_waked(|cx| {
        let change = cx.use_dual_query_set_raw(*e_id).map(|v| v.delta);

        changes_collector.push(EntityDeletionChangeCollecting {
          entity_id: *e_id,
          change,
        });
      });
    }
  });

  if cx.is_spawning_stage() {
    let mut changes_to_waits = Vec::default();
    for c in changes_collector {
      let entity_id = c.entity_id;
      if let Some(change) = c.change.into_spawn_stage_future() {
        changes_to_waits.push(change.map(move |change| EntityDeletionChange { entity_id, change }));
      }
    }
    UseResult::SpawnStageFuture(pin_box_in_frame(
      join_all(changes_to_waits).map(|v| Arc::new(v)),
    ))
  } else {
    UseResult::NotInStage
  }
}
