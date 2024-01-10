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
  pending_upstream: Weak<MapPollFuture>,
}

impl<Map, K, V> ReactiveKVMapFork2<Map, K, V> {
  // pub fn get_or_create_upstream_work(&self) -> impl Stream<Item = ()>;
}

pub enum MaybeResolved<F: Future> {
  Future(F),
  Resolved(F::Output),
}

#[derive(Clone)]
struct MapPollFuture<K, V> {
  upstream: MaybeResolved<CollectionTransactionFuture<'static, K, V>>,
  all_downstream: Weak<RwLock<FastHashMap<u64, DownStreamInfo2<K, V>>>>,
}

impl<K, V> Future for Arc<MapPollFuture<K, V>> {
  type Output = ();

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
      // previous buffered
      self.rev.next().await;

      // new upstream
      self.rev.next().await;

      // merge
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
