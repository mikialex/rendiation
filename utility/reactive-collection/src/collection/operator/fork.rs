use std::future::ready;

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
    todo!()
    // // get updated current view and delta; this is necessary, because we require
    // // current view as init delta for new forked downstream
    // //
    // // update should use downstream registered waker
    // let waker = Arc::new(BroadCast {
    //   inner: self.downstream.clone(),
    // });
    // let waker = futures::task::waker_ref(&waker);
    // let mut cx = Context::from_waker(&waker);
    // let (d, v) = self.upstream.read().poll_changes(&mut cx);

    // // delta should also dispatch to downstream to keep message integrity
    // let downstream = self.downstream.read_recursive();
    // let d = d.materialize();
    // for (_, info) in downstream.iter() {
    //   if info.should_send {
    //     info.sender.unbounded_send(d.clone()).ok();
    //   }
    // }

    // // now we create new downstream
    // let mut downstream = self.downstream.write();
    // let id = alloc_global_res_id();
    // let (sender, rev) = unbounded();

    // // pass the current table as the init change
    // let current = v
    //   .iter_key_value()
    //   .map(|(k, v)| (k, ValueChange::Delta(v, None)))
    //   .collect::<FastHashMap<_, _>>();
    // let current = Arc::new(current);
    // if !current.is_empty() {
    //   sender.unbounded_send(current).ok();
    // }

    // let waker: Arc<AtomicWaker> = Default::default();
    // let info = DownStreamInfo {
    //   waker: waker.clone(),
    //   sender,
    //   should_send: true,
    // };

    // downstream.insert(id, info);

    // Self {
    //   upstream: self.upstream.clone(),
    //   downstream: self.downstream.clone(),
    //   id,
    //   rev: RwLock::new(rev),
    //   waker,
    //   phantom: PhantomData,
    //   buffered: Default::default(),
    // }
  }
}

fn finalize_buffered_changes<K: CKey, V: CValue>(
  mut changes: Vec<ForkMessage<K, V>>,
) -> Box<dyn DynVirtualCollection<K, ValueChange<V>>> {
  if changes.is_empty() {
    return Box::new(());
  }

  if changes.len() == 1 {
    let first = changes.pop().unwrap();
    if first.is_empty() {
      return Box::new(());
    } else {
      return Box::new(first);
    }
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(&mut target, c.iter().map(|(k, v)| (k.clone(), v.clone())));
  }

  if target.is_empty() {
    Box::new(())
  } else {
    Box::new(target)
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = Map::View;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;
  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    // // install new waker, this waker is shared by arc within the downstream info
    // self.waker.register(cx.waker());

    // // read and merge all possible forked buffered messages from channel
    // let mut buffered = std::mem::take(self.buffered.write().deref_mut());

    // while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(cx) {
    //   buffered.push(changes);
    // }

    // // we have to also check the upstream no matter if we have message in channel or not
    // // todo, exponential check cost if we have share rich structure in graph
    // let upstream = self.upstream.write();
    // let waker = Arc::new(BroadCast {
    //   inner: self.downstream.clone(),
    // });
    // let waker = futures::task::waker_ref(&waker);
    // let mut cx_2 = Context::from_waker(&waker);
    // let (d, v) = upstream.poll_changes(&mut cx_2);

    // let d = {
    //   let downstream = self.downstream.write();
    //   let d = d.materialize();
    //   if !d.is_empty() {
    //     // broad cast to others
    //     // we are not required to call broadcast waker because it will be waked by others
    //     // receivers
    //     for (id, downstream) in downstream.iter() {
    //       if *id != self.id && downstream.should_send {
    //         downstream.sender.unbounded_send(d.clone()).ok();
    //       }
    //     }

    //     buffered.push(d);
    //   }
    //   drop(downstream);

    //   finalize_buffered_changes(buffered)
    // };

    // (d, v)

    self.upstream.read().poll_changes(cx).map(|(d, v)| (d, v))
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
