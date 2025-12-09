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

pub trait AbstractUriHookCx: QueryHookCxLike {
  fn uri_source<P: UriProvider>(&mut self) -> Arc<RwLock<dyn UriDataSourceDyn<P::Data>>>;

  fn use_maybe_uri_data_changes<P, C>(
    &mut self,
    changes: UseResult<C>,
  ) -> UseResult<impl DataChanges<Value = P::Data>>
  where
    P: UriProvider,
    C: DataChanges<Value = MaybeUriData<P::Data>> + 'static,
    P::Data: CValue,
  {
    let data_source = self.uri_source::<P>();
    let (_, futures) = self.use_plain_state_default_cloned::<Arc<
      RwLock<
        MappedFutures<C::Key, Box<dyn Future<Output = Option<P::Data>> + Send + Sync + Unpin>>,
      >,
    >>();

    let waker = self.waker().clone();

    changes.map(move |changes| {
      let mut data_source = data_source.write(); // todo, this may deadlock
      let mut futures = futures.write();
      let futures = &mut *futures;
      // do cancelling first
      for removed in changes.iter_removed() {
        futures.remove(&removed);
      }

      // although the changes insert list may duplicate, it is not a problem
      for (k, v) in changes.iter_update_or_insert() {
        if let MaybeUriData::Uri(uri) = v {
          let fut = data_source.request_uri_data_load(&uri);
          futures.replace(k, fut);
        }
      }

      let mut loaded_list = Vec::new();
      let mut ctx = Context::from_waker(&waker);
      while let Poll::Ready(Some((key, loaded))) = futures.poll_next_unpin(&mut ctx) {
        if let Some(loaded) = loaded {
          loaded_list.push((key, loaded))
        }
      }

      changes.collective_filter_map(|v| v.into_living()) // todo, append loaded_list
    })
  }
}
