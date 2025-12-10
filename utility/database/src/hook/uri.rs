use std::{
  future::Future,
  ops::Deref,
  sync::Arc,
  task::{Context, Poll},
};

use futures::stream::StreamExt;
use mapped_futures::MappedFutures; // todo, i don't like this dep
use parking_lot::RwLock;
use query::*;
use query_hook::*;
pub use rendiation_abstract_uri_data::*;

use crate::ExternalRefPtr;

/// this trait is to reserve the design space for virtualization related scheduling logic
pub trait AbstractResourceScheduler: Send + Sync + 'static {
  type Data;
  type Key;

  fn notify_use_resource(&mut self, key: &Self::Key, uri: &str);
  fn notify_remove_resource(&mut self, key: &Self::Key);

  /// the consumer must do dropping first, to make sure the peak memory usage is in bound
  fn poll_schedule(&mut self, cx: &mut Context) -> LinearBatchChanges<Self::Key, Self::Data>;
}

/// the basic implementation is load what your request to load
pub struct NoScheduleScheduler<K: CKey, V> {
  pub futures: MappedFutures<K, Box<dyn Future<Output = Option<V>> + Send + Sync + Unpin>>,
  pub data_source: Box<dyn UriDataSourceDyn<V>>,
}

impl<K: CKey, V: CValue> AbstractResourceScheduler for NoScheduleScheduler<K, V> {
  type Data = V;
  type Key = K;

  fn notify_use_resource(&mut self, key: &Self::Key, uri: &str) {
    let future = self.data_source.request_uri_data_load(uri);
    self.futures.replace(key.clone(), future);
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.futures.remove(key);
  }

  fn poll_schedule(&mut self, cx: &mut Context) -> LinearBatchChanges<Self::Key, Self::Data> {
    let mut load_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      if let Some(loaded) = loaded {
        load_list.push((key, loaded))
      }
    }

    LinearBatchChanges {
      removed: Vec::new(), // this can be empty, because it will removed by caller anyway
      update_or_insert: load_list,
    }
  }
}

// todo, optimize impl
pub fn use_maybe_uri_data_changes<P, C>(
  cx: &mut impl QueryHookCxLike,
  changes: UseResult<C>,
  scheduler: &Arc<RwLock<P>>,
) -> UseResult<LinearBatchChanges<P::Key, Option<P::Data>>>
where
  P: AbstractResourceScheduler,
  C: DataChanges<Key = P::Key, Value = Option<ExternalRefPtr<MaybeUriData<P::Data>>>> + 'static,
  P::Data: CValue,
  P::Key: CKey,
{
  let scheduler = scheduler.clone();
  let waker = cx.waker().clone();

  changes.map_spawn_stage_in_thread(
    cx,
    |changes| changes.has_change(),
    move |changes| {
      // todo, we should use some async lock to avoid blocking
      let mut scheduler = scheduler.write();

      let mut all_removed = Vec::new();
      // do cancelling first
      // the futures should not resolved in poll next call
      for removed in changes.iter_removed() {
        scheduler.notify_remove_resource(&removed);
        all_removed.push(removed);
      }

      // let mut new_inserted = Vec::new();

      // although the changes insert list may duplicate, it is not a problem but will have some performance cost
      for (k, v) in changes.iter_update_or_insert() {
        if let Some(v) = v {
          let v = v.deref();
          match v {
            MaybeUriData::Uri(uri) => {
              scheduler.notify_use_resource(&k, &uri);
            }
            MaybeUriData::Living(v) => {
              // new_inserted.push((k, v));
            }
          }
        }
      }

      // let mut ctx = Context::from_waker(&waker);
      // let loading = data_scheduler.poll_schedule(&mut ctx);

      // new_inserted.extend(loading.update_or_insert);
      // all_removed.extend(loading.removed);

      // LinearBatchChanges {
      //   removed: all_removed,
      //   update_or_insert: new_inserted,
      // };

      todo!()
    },
  )
}
