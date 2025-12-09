use std::{
  future::Future,
  sync::Arc,
  task::{Context, Poll},
};

use futures::stream::StreamExt;
use mapped_futures::MappedFutures; // todo, i don't like this dep
use parking_lot::RwLock;
use query::*;
use query_hook::*;
pub use rendiation_abstract_uri_data::*;

/// this trait is to reserve the design space for virtualization related scheduling logic
pub trait AbstractSourceScheduler: Send + Sync + 'static {
  type Data;
  type Key;

  fn notify_use_resource(&mut self, key: &Self::Key, uri: &str);
  fn notify_remove_resource(&mut self, key: &Self::Key);

  fn poll_schedule(&mut self, cx: &mut Context) -> Vec<(Self::Key, Self::Data)>;
}

/// the basic implementation is load what your request to load
pub struct NoScheduleScheduler<K: CKey, V> {
  pub futures: MappedFutures<K, Box<dyn Future<Output = Option<V>> + Send + Sync + Unpin>>,
  pub data_source: Box<dyn UriDataSourceDyn<V>>,
}

impl<K: CKey, V: CValue> AbstractSourceScheduler for NoScheduleScheduler<K, V> {
  type Data = V;
  type Key = K;

  fn notify_use_resource(&mut self, key: &Self::Key, uri: &str) {
    let future = self.data_source.request_uri_data_load(uri);
    self.futures.replace(key.clone(), future);
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.futures.remove(key);
  }

  fn poll_schedule(&mut self, cx: &mut Context) -> Vec<(Self::Key, Self::Data)> {
    let mut loaded_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      if let Some(loaded) = loaded {
        loaded_list.push((key, loaded))
      }
    }

    loaded_list
  }
}

pub trait AbstractUriHookCx: QueryHookCxLike {
  fn uri_source<P: AbstractSourceScheduler>(&mut self) -> Arc<RwLock<P>>;

  fn use_maybe_uri_data_changes<P, C>(
    &mut self,
    changes: UseResult<C>,
  ) -> UseResult<impl DataChanges<Value = P::Data>>
  where
    P: AbstractSourceScheduler,
    C: DataChanges<Key = P::Key, Value = MaybeUriData<P::Data>> + 'static,
    P::Data: CValue,
  {
    let data_scheduler = self.uri_source::<P>();

    let waker = self.waker().clone();

    changes.map_spawn_stage_in_thread(
      self,
      |changes| changes.has_change(),
      move |changes| {
        // todo, we should use some async lock to avoid blocking
        let mut data_scheduler = data_scheduler.write();

        // do cancelling first
        // the futures should not resolved in poll next call
        for removed in changes.iter_removed() {
          data_scheduler.notify_remove_resource(&removed);
        }

        // although the changes insert list may duplicate, it is not a problem but will have some performance cost
        for (k, v) in changes.iter_update_or_insert() {
          if let MaybeUriData::Uri(uri) = v {
            data_scheduler.notify_use_resource(&k, &uri);
          }
        }

        let mut ctx = Context::from_waker(&waker);
        let loading = data_scheduler.poll_schedule(&mut ctx);

        changes.collective_filter_map(|v| v.into_living()) // todo, append loaded_list
      },
    )
  }
}
