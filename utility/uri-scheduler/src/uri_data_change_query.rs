use crate::*;

pub trait SchedulerReInitIteratorProvider {
  type Item;
  fn create_iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_>;
}

pub struct DataChangesAndLivingReInit<K, VLiving, VUrl> {
  pub changes: Arc<LinearBatchChanges<K, MaybeUriData<VLiving, VUrl>>>,
  pub iter_living_full: Arc<dyn SchedulerReInitIteratorProvider<Item = (K, VLiving)> + Send + Sync>,
}

impl<K, VLiving, VUrl> Clone for DataChangesAndLivingReInit<K, VLiving, VUrl> {
  fn clone(&self) -> Self {
    Self {
      changes: self.changes.clone(),
      iter_living_full: self.iter_living_full.clone(),
    }
  }
}

type Reconciler<K, V> = FastHashMap<u32, Vec<Arc<LinearBatchChanges<K, UriLoadResult<V>>>>>;
type SharedReconciler<K, V> = Arc<RwLock<Reconciler<K, V>>>;

// todo, optimize impl
/// the scheduler should not be shared between different use_maybe_uri_data_changes
/// the Arc is only to support it access in thread
pub fn use_uri_data_changes<P, Cx: QueryHookCxLike>(
  cx: &mut Cx,
  source: impl SharedResultProvider<
    Cx,
    Result = DataChangesAndLivingReInit<P::Key, P::Data, P::UriLike>,
  >,
  scheduler: &Arc<RwLock<P>>,
  loader_creator: LoaderCreator<P::UriLike, P::Data>,
) -> UseResult<Arc<LinearBatchChanges<P::Key, UriLoadResult<P::Data>>>>
where
  P: AbstractResourceScheduler + 'static,
  P::Data: Send + Sync + 'static + Clone,
  P::UriLike: Send + Sync + 'static + Clone,
  P::Key: CKey,
{
  let share_key = source.compute_share_key();
  let debug_label = source.debug_label();
  let consumer_id = cx.use_shared_consumer(share_key);

  let all_downstream_changes = cx.use_shared_compute_internal(
    &|cx| {
      let changes = source.use_logic(cx);

      let scheduler = scheduler.clone();
      let waker = cx.waker().clone();

      let (cx, reconciler) =
        cx.use_plain_state_default_cloned::<SharedReconciler<P::Key, P::Data>>();

      let loader_creator = loader_creator.clone();

      let re = changes.map_spawn_stage_in_thread(
        cx,
        |changes| changes.changes.has_change(),
        move |changes| {
          let mut scheduler = scheduler.write();

          let mut all_removed = Vec::new();
          // do cancelling first
          // the futures should not resolved in poll next call
          for removed in changes.changes.iter_removed() {
            scheduler.notify_remove_resource(&removed);
            all_removed.push(removed);
          }

          let mut new_inserted = Vec::new();
          let mut new_loading = fast_hash_collection::FastHashSet::default();

          let mut loader = loader_creator();

          // although the changes insert list may duplicate, it is not a problem but will have some performance cost
          for (k, v) in changes.changes.iter_update_or_insert() {
            match v {
              MaybeUriData::Uri(uri) => {
                scheduler.notify_use_resource(&k, &uri, &mut loader);
                new_loading.insert(k.clone());
              }
              MaybeUriData::Living(v) => {
                new_inserted.push((k, UriLoadResult::LivingOrLoaded(v.clone())));
              }
            }
          }

          let mut ctx = Context::from_waker(&waker);
          let loaded = scheduler.poll_schedule(&mut ctx, &mut loader);

          for (k, v) in loaded.iter_update_or_insert() {
            new_loading.remove(&k);
            if let Some(v) = v {
              new_inserted.push((k, UriLoadResult::LivingOrLoaded(v)));
            } else {
              new_inserted.push((k, UriLoadResult::PresentButFailedToLoad));
            }
          }

          for k in new_loading {
            new_inserted.push((k, UriLoadResult::PresentButNotLoaded));
          }

          let after_schedule_changes = Arc::new(LinearBatchChanges {
            removed: all_removed,
            update_or_insert: new_inserted,
          });

          {
            let mut r = reconciler.write();
            for downstream in r.values_mut() {
              downstream.push(after_schedule_changes.clone());
            }
          }

          (reconciler, changes.iter_living_full)
        },
      );

      re
    },
    share_key,
    debug_label,
    consumer_id,
  );

  let (cx, cleanup) =
    cx.use_plain_state_default_cloned::<Arc<RwLock<Option<Cleanup<P::Key, P::Data>>>>>();

  let scheduler = scheduler.clone();
  all_downstream_changes.map_spawn_stage_in_thread(
    cx,
    move |(reconciler, _)| {
      let r = reconciler.read();
      if let Some(buffered_changes) = r.get(&consumer_id) {
        buffered_changes.len() > 1
      } else {
        let mut cleanup = cleanup.write();
        if cleanup.is_none() {
          *cleanup = Some(Cleanup(reconciler.clone(), consumer_id));
        }
        true
      }
    },
    move |(reconciler, init_iter)| {
      let mut reconciler = reconciler.write();
      let messages = reconciler.entry(consumer_id).or_insert_with(|| {
        scheduler.write().reload_all_loaded();
        let init_iter = init_iter.create_iter();
        let init_message = Arc::new(LinearBatchChanges {
          removed: Default::default(),
          update_or_insert: init_iter
            .map(|(k, v)| (k, UriLoadResult::LivingOrLoaded(v)))
            .collect(),
        });
        vec![init_message]
      });

      merge_linear_batch_changes(messages)
    },
  )
}

struct Cleanup<K, V>(SharedReconciler<K, V>, u32);
impl<K, V> Drop for Cleanup<K, V> {
  fn drop(&mut self) {
    let removed = self.0.write().remove(&self.1);
    assert!(removed.is_some());
  }
}
