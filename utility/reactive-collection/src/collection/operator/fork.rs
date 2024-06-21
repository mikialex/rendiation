use futures::channel::mpsc::*;

use crate::*;

type ForkMessage<K, V> = Arc<FastHashMap<K, ValueChange<V>>>;

type Sender<K, V> = UnboundedSender<ForkMessage<K, V>>;
type Receiver<K, V> = UnboundedReceiver<ForkMessage<K, V>>;

struct DownStreamInfo<K, V> {
  waker: Arc<AtomicWaker>,
  sender: Sender<K, V>,
  /// some fork never receive message just act as a static forker, in this case the message should
  /// not send to it to avoid memory leak.
  should_send: bool,
}

pub struct ReactiveKVMapFork<Map, K, V> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<K, V>>>>,
  buffered: RwLock<Vec<ForkMessage<K, V>>>,
  rev: RwLock<Receiver<K, V>>,
  id: u64,
  waker: Arc<AtomicWaker>,
  phantom: PhantomData<(K, V)>,
}

impl<Map, K, V> ReactiveKVMapFork<Map, K, V> {
  pub fn new(upstream: Map, as_static_forker: bool) -> Self {
    let (sender, rev) = unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    let info = DownStreamInfo {
      waker: waker.clone(),
      sender,
      should_send: !as_static_forker,
    };
    init.insert(id, info);
    ReactiveKVMapFork {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Arc::new(RwLock::new(init)),
      rev: RwLock::new(rev),
      id,
      waker,
      phantom: Default::default(),
      buffered: Default::default(),
    }
  }
}

impl<Map, K, V> Drop for ReactiveKVMapFork<Map, K, V> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}

impl<K, V, Map> Clone for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn clone(&self) -> Self {
    // when fork the collection, we should pass the current table as the init change
    let upstream = self.upstream.read_recursive();

    let u: &Map = &upstream;
    let current = u
      .access()
      .iter_key_value()
      .map(|(k, v)| (k, ValueChange::Delta(v, None)))
      .collect::<FastHashMap<_, _>>();
    let current = Arc::new(current);

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    let (sender, rev) = unbounded();

    if !current.is_empty() {
      sender.unbounded_send(current).ok();
    }

    let waker: Arc<AtomicWaker> = Default::default();
    let info = DownStreamInfo {
      waker: waker.clone(),
      sender,
      should_send: true,
    };

    downstream.insert(id, info);

    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      id,
      rev: RwLock::new(rev),
      waker,
      phantom: PhantomData,
      buffered: Default::default(),
    }
  }
}

fn finalize_buffered_changes<K: CKey, V: CValue>(
  mut changes: Vec<ForkMessage<K, V>>,
) -> PollCollectionChanges<K, V> {
  if changes.is_empty() {
    return Poll::Pending;
  }

  if changes.len() == 1 {
    let first = changes.pop().unwrap();
    if first.is_empty() {
      return Poll::Pending;
    } else {
      return Poll::Ready(Box::new(first));
    }
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(&mut target, c.iter().map(|(k, v)| (k.clone(), v.clone())));
  }

  if target.is_empty() {
    Poll::Pending
  } else {
    Poll::Ready(Box::new(target))
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    if self.waker.take().is_some() {
      self.waker.register(cx.waker());
      // the previous installed waker not waked, nothing changes, directly return
      return Poll::Pending;
    }
    // install new waker
    self.waker.register(cx.waker());

    // read and merge all possible forked buffered messages from channel
    let mut buffered = std::mem::take(self.buffered.write().deref_mut());

    while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(cx) {
      buffered.push(changes);
    }

    // we have to also check the upstream no matter if we have message in channel or not
    let upstream = self.upstream.write();
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx_2 = Context::from_waker(&waker);
    let r = upstream.poll_changes(&mut cx_2);

    match r {
      Poll::Ready(v) => {
        let downstream = self.downstream.write();
        let c = v.materialize();
        if !c.is_empty() {
          // broad cast to others
          // we are not required to call broadcast waker because it will be waked by others
          // receivers
          for (id, downstream) in downstream.iter() {
            if *id != self.id && downstream.should_send {
              downstream.sender.unbounded_send(c.clone()).ok();
            }
          }

          buffered.push(c);
        }
        drop(downstream);

        finalize_buffered_changes(buffered)
      }
      Poll::Pending => finalize_buffered_changes(buffered),
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.upstream.read().access()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

/// notify all downstream proactively
struct BroadCast<K, V> {
  inner: Arc<RwLock<FastHashMap<u64, DownStreamInfo<K, V>>>>,
}

impl<K: CKey, V: CValue> futures::task::ArcWake for BroadCast<K, V> {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    let all = arc_self.inner.write();
    for v in all.values() {
      v.waker.wake();
    }
  }
}

impl<Map, K, V> ReactiveOneToManyRelation<V, K> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveOneToManyRelation<V, K>,
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CKey,
{
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<V, K>> {
    self.upstream.read().multi_access()
  }
}
