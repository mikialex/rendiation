use std::sync::Weak;

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
  resolved: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
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
      resolved: Default::default(),
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
    // get updated current view and delta; this is necessary, because we require
    // current view as init delta for new forked downstream
    //
    // update should use downstream registered waker
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx = Context::from_waker(&waker);
    let (d, v) = self.upstream.read().poll_changes(&mut cx);

    // delta should also dispatch to downstream to keep message integrity
    let downstream = self.downstream.read_recursive();
    let d = d.materialize();
    for (_, info) in downstream.iter() {
      if info.should_send {
        info.sender.unbounded_send(d.clone()).ok();
      }
    }
    drop(downstream);

    // now we create new downstream
    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    let (sender, rev) = unbounded();

    // pass the current table as the init change
    let current = v
      .iter_key_value()
      .map(|(k, v)| (k, ValueChange::Delta(v, None)))
      .collect::<FastHashMap<_, _>>();
    let current = Arc::new(current);
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
      resolved: self.resolved.clone(),
      waker,
      phantom: PhantomData,
      buffered: Default::default(),
    }
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

#[derive(Clone)]
pub struct ForkedView<T> {
  inner: Arc<T>,
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V>> VirtualCollection<K, V> for ForkedView<T> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.inner.iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    self.inner.access(key)
  }
}
impl<K: CKey, V: CKey, T: VirtualMultiCollection<K, V>> VirtualMultiCollection<K, V>
  for ForkedView<T>
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_> {
    self.inner.access_multi(key)
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = ForkedView<Map::View>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    // install new waker, this waker is shared by arc within the downstream info
    self.waker.register(cx.waker());

    // read and merge all possible forked buffered messages from channel
    let mut buffered = std::mem::take(self.buffered.write().deref_mut());

    while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(cx) {
      buffered.push(changes);
    }

    // if we have any upstream polled but not dropped, we directly return the cached resolved view
    {
      let resolved = self.resolved.read();
      let resolved: &Option<Weak<dyn Any + Send + Sync>> = &resolved;
      if let Some(view) = resolved {
        if let Some(view) = view.upgrade() {
          let v = ForkedView {
            inner: view.downcast::<Map::View>().unwrap(),
          };
          return (finalize_buffered_changes(buffered), v);
        }
      }
    }

    // poll upstream
    let upstream = self.upstream.write();
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx_2 = Context::from_waker(&waker);
    let (d, v) = upstream.poll_changes(&mut cx_2);

    let d = {
      let downstream = self.downstream.write();
      let d = d.materialize();
      if !d.is_empty() {
        // broad cast to others
        // we are not required to call broadcast waker because it will be waked by others
        // receivers
        for (id, downstream) in downstream.iter() {
          if *id != self.id && downstream.should_send {
            downstream.sender.unbounded_send(d.clone()).ok();
          }
        }

        buffered.push(d);
      }
      drop(downstream);

      finalize_buffered_changes(buffered)
    };

    let v = Arc::new(v) as Arc<dyn Any + Send + Sync>;
    let view = v.clone();
    *self.resolved.write() = Some(Arc::downgrade(&v));

    let v = ForkedView {
      inner: view.downcast::<Map::View>().unwrap(),
    };

    (d, v)
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
