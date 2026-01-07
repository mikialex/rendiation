use std::{
  future::Future,
  sync::Arc,
  task::{Context, Poll},
};

use fast_hash_collection::FastHashMap;
use futures::stream::StreamExt;
use mapped_futures::MappedFutures; // todo, i don't like this dep
use parking_lot::RwLock;
use query::*;
use query_hook::*;
pub use rendiation_abstract_uri_data::*;

/// this trait is to reserve the design space for virtualization related scheduling logic
pub trait AbstractResourceScheduler: Send + Sync {
  type UriLike;
  type Data;
  type Key;

  fn notify_use_resource(
    &mut self,
    key: &Self::Key,
    uri: &Self::UriLike,
    loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  );
  fn notify_remove_resource(&mut self, key: &Self::Key);

  /// this api is to support dynamic creation of downstream, when a new downstream has created,
  /// we have to reload all loaded data. to make sure the new downstream has correct message received,
  /// and all downstream has same message witnessed.
  ///
  /// a better behavior may implemented but it will greatly complicated the implementation. reload all is a acceptable tradeoff
  /// for this rare case.
  fn reload_all_loaded(&mut self);

  /// the consumer must do dropping first, to make sure the peak memory usage is in bound
  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  ) -> LinearBatchChanges<Self::Key, Self::Data>;
}

impl<K: CKey, V: CValue, U> AbstractResourceScheduler
  for Box<dyn AbstractResourceScheduler<Data = V, Key = K, UriLike = U>>
{
  type UriLike = U;
  type Data = V;
  type Key = K;

  fn notify_use_resource(
    &mut self,
    key: &Self::Key,
    uri: &Self::UriLike,
    loader: &mut LoaderFunction<U, V>,
  ) {
    (**self).notify_use_resource(key, uri, loader);
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    (**self).notify_remove_resource(key);
  }

  fn reload_all_loaded(&mut self) {
    (**self).reload_all_loaded();
  }

  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<U, V>,
  ) -> LinearBatchChanges<Self::Key, Self::Data> {
    (**self).poll_schedule(cx, loader)
  }
}

pub type LoadFuture<T> = Box<dyn Future<Output = Option<T>> + Send + Sync + Unpin>;
pub type LoaderFunction<K, T> = dyn FnMut(&K) -> LoadFuture<T> + Send + Sync;
pub type LoaderCreator<K, T> = Arc<dyn Fn() -> Box<LoaderFunction<K, T>> + Send + Sync>;

/// the basic implementation is load what your request to load
pub struct NoScheduleScheduler<K: CKey, V, URI> {
  pub futures: MappedFutures<K, LoadFuture<V>>,
  pub loading_uri: FastHashMap<K, URI>,
  pub loaded: FastHashMap<K, URI>,
  pub request_reload: bool,
}

impl<K: CKey, V, URI> Default for NoScheduleScheduler<K, V, URI> {
  fn default() -> Self {
    Self {
      futures: MappedFutures::new(),
      loaded: FastHashMap::default(),
      request_reload: false,
      loading_uri: FastHashMap::default(),
    }
  }
}

impl<K: CKey, V, URI: Clone + Send + Sync> AbstractResourceScheduler
  for NoScheduleScheduler<K, V, URI>
{
  type Data = V;
  type Key = K;
  type UriLike = URI;

  fn notify_use_resource(
    &mut self,
    key: &Self::Key,
    uri: &URI,
    loader: &mut LoaderFunction<URI, V>,
  ) {
    let future = loader(uri);
    self.futures.replace(key.clone(), future);
    self.loading_uri.insert(key.clone(), uri.clone());
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.futures.remove(key);
    self.loaded.remove(key);
  }

  fn reload_all_loaded(&mut self) {
    self.request_reload = true;
  }

  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<URI, V>,
  ) -> LinearBatchChanges<Self::Key, Self::Data> {
    if self.request_reload {
      self.request_reload = false;
      for (key, uri) in &self.loaded {
        let future = loader(uri);
        self.futures.replace(key.clone(), future);
        self.loading_uri.insert(key.clone(), uri.clone());
      }
    }

    let mut load_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      if let Some(loaded) = loaded {
        load_list.push((key.clone(), loaded));
        let uri = self.loading_uri.remove(&key).unwrap();
        self.loaded.insert(key, uri);
      }
    }

    LinearBatchChanges {
      removed: Vec::new(), // this can be empty, because it will removed by caller anyway
      update_or_insert: load_list,
    }
  }
}

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

type Reconciler<K, V> = FastHashMap<u32, Vec<Arc<LinearBatchChanges<K, Option<V>>>>>;
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
) -> UseResult<Arc<LinearBatchChanges<P::Key, Option<P::Data>>>>
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
                new_inserted.push((k, Some(v.clone())));
              }
            }
          }

          let mut ctx = Context::from_waker(&waker);
          let loaded = scheduler.poll_schedule(&mut ctx, &mut loader);

          for (k, v) in loaded.iter_update_or_insert() {
            new_loading.remove(&k);
            new_inserted.push((k, Some(v)));
          }

          for k in new_loading {
            new_inserted.push((k, None));
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
          update_or_insert: init_iter.map(|(k, v)| (k, Some(v))).collect(),
        });
        vec![init_message]
      });

      let merged = merge_linear_batch_changes(messages);
      messages.clear();
      Arc::new(merged)
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
