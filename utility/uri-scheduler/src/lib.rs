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

mod scheduler;
pub use scheduler::*;

mod no_scheduler;
pub use no_scheduler::*;

mod uri_data_change_query;
pub use uri_data_change_query::*;

mod loading_throttler;
pub use loading_throttler::*;

pub struct ResourceMemoryCost {
  pub retain: u64,
  pub loading_peak: u64,
}

pub trait ProvideMemoryCostInfo {
  fn memory_cost(&self) -> ResourceMemoryCost;
}

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
  ///
  /// if load result is none, it means the item is scheduled in but failed to load
  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  ) -> LinearBatchChanges<Self::Key, Option<Self::Data>>;
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
  ) -> LinearBatchChanges<Self::Key, Option<Self::Data>> {
    (**self).poll_schedule(cx, loader)
  }
}

/// if the output is None, it means the data is failed to load
pub type LoadFuture<T> = Box<dyn Future<Output = Option<T>> + Send + Sync + Unpin>;
pub type LoaderFunction<K, T> = dyn FnMut(&K) -> LoadFuture<T> + Send + Sync;
pub type LoaderCreator<K, T> = Arc<dyn Fn() -> Box<LoaderFunction<K, T>> + Send + Sync>;
