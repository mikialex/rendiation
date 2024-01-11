use std::sync::Weak;

use futures::task::AtomicWaker;

use crate::*;

pub struct CollectionTransaction<'a, K, V> {
  view: Box<dyn VirtualCollection<K, V> + 'a>,
  delta: Box<dyn VirtualCollection<K, ValueChange<V>> + 'a>,
}

pub type CollectionTransactionFuture<'a, K, V> =
  Box<dyn Future<Output = CollectionTransaction<'a, K, V>>>;

pub trait ReactiveCollection2<K: CKey, V: CValue>: Sync + Send + 'static {
  fn poll_changes(&self, cx: &mut Context) -> CollectionTransactionFuture<K, V>;
}

pub struct ReactiveKVMapFork2<Map, K, V> {
  upstream: Arc<Map>,
  all_downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo2<K, V>>>>,
  rev: RwLock<Receiver<K, V>>,
  id: u64,
  waker: Arc<AtomicWaker>,
  pending_upstream: RwLock<Weak<MapPollFutureImpl<K, V>>>,
}

impl<Map, K, V> ReactiveKVMapFork2<Map, K, V> {
  pub fn get_or_create_upstream_work(&self, cx: &mut Context) -> MapPollFuture<K, V> {
    let pending_upstream = self.pending_upstream.write();
    if let Some(upstream_future) = pending_upstream.upgrade() {
      //
    } else {
      *pending_upstream = self.upstream.poll_changes(cx);
    }
  }
}

pub enum MaybeResolved<F: Future> {
  Future(F),
  Resolved(F::Output),
}

#[derive(Clone)]
struct MapPollFutureImpl<K, V> {
  upstream: MaybeResolved<CollectionTransactionFuture<'static, K, V>>,
  sended: Weak<RwLock<FastHashSet<u64>>>,
  all_downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo2<K, V>>>>,
}

impl<K, V> Drop for MapPollFutureImpl<K, V> {
  fn drop(&mut self) {
    // todo
    // send all not sended to downstream channels
  }
}

struct MapPollFuture<K, V> {
  inner: Arc<MapPollFutureImpl<K, V>>,
  id: u64,
}

impl<K, V> Future for MapPollFuture<K, V> {
  type Output = Box<dyn VirtualCollection<K, V>>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    todo!()
  }
}

impl<Map, K, V> ReactiveCollection2<K, V> for ReactiveKVMapFork2<Map, K, V>
where
  K: CKey,
  V: CValue,
  Map: ReactiveCollection2<K, V>,
{
  fn poll_changes(&self, cx: &mut Context) -> CollectionTransactionFuture<K, V> {
    async {
      let view = self.get_or_create_upstream_task().await;

      let delta = self.all_buffered_changes().await;

      CollectionTransaction { view, delta }
    };
    todo!()
  }
}

struct DownStreamInfo2<K, V> {
  waker: Arc<AtomicWaker>,
  sender: Sender<K, V>,
  /// some fork never receive message just act as a static forker, in this case the message should
  /// not send to it to avoid memory leak.
  should_send: bool,
}
