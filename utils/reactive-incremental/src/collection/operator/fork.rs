use std::ops::DerefMut;
use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::FastHashMap;
use futures::channel::mpsc::*;
use futures::task::AtomicWaker;
use parking_lot::RwLockReadGuard;

use crate::*;

type ForkMessage<K, V> = FastHashMap<K, ValueChange<V>>;

type Sender<K, V> = UnboundedSender<ForkMessage<K, V>>;
type Receiver<K, V> = UnboundedReceiver<ForkMessage<K, V>>;

pub struct ReactiveKVMapFork<Map, K, V> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, (Arc<AtomicWaker>, Sender<K, V>)>>>,
  buffered: RwLock<ForkMessage<K, V>>,
  rev: RwLock<Receiver<K, V>>,
  id: u64,
  waker: Arc<AtomicWaker>,
  phantom: PhantomData<(K, V)>,
}

impl<Map, K, V> ReactiveKVMapFork<Map, K, V> {
  pub fn new(upstream: Map) -> Self {
    let (sender, rev) = unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    init.insert(id, (waker.clone(), sender));
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
    let current = u.spin_get_current();
    let current = current
      .iter_key_value()
      .map(|(k, v)| (k, ValueChange::Delta(v, None)))
      .collect::<FastHashMap<_, _>>();

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    let (sender, rev) = unbounded();

    if !current.is_empty() {
      sender.unbounded_send(current).ok();
    }

    let waker: Arc<AtomicWaker> = Default::default();
    downstream.insert(id, (waker.clone(), sender));

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

pub fn poll_and_merge_all<K: CKey, V: CValue>(
  cx: &mut Context,
  source: &mut (impl Stream<Item = ForkMessage<K, V>> + Unpin),
) -> FastHashMap<K, ValueChange<V>> {
  let mut buffered: FastHashMap<K, ValueChange<V>> = Default::default();

  while let Poll::Ready(Some(changes)) = source.poll_next_unpin(cx) {
    merge_into_hashmap(&mut buffered, changes.into_iter())
  }

  buffered
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
      return CPoll::Ready(Poll::Pending);
    }
    // install new waker
    self.waker.register(cx.waker());

    // read and merge all possible forked buffered messages from channel
    let mut buffered = std::mem::take(self.buffered.write().deref_mut());

    while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(cx) {
      merge_into_hashmap(&mut buffered, changes.into_iter())
    }

    // we have to also check the upstream no matter if we have message in channel or not
    if let Some(upstream) = self.upstream.try_write() {
      let waker = Arc::new(BroadCast {
        inner: self.downstream.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx_2 = Context::from_waker(&waker);
      let r = upstream.poll_changes(&mut cx_2);

      match r {
        CPoll::Ready(v) => match v {
          Poll::Ready(v) => {
            let downstream = self.downstream.write();
            let c = v.materialize_hashmap_maybe_cloned();
            if !c.is_empty() {
              // broad cast to others
              // we are not required to call broadcast waker because it will be waked by others
              // receivers
              for (id, downstream) in downstream.iter() {
                if *id != self.id {
                  downstream.1.unbounded_send(c.clone()).ok();
                }
              }

              merge_into_hashmap(&mut buffered, c.into_iter())
            }
            drop(downstream);

            if buffered.is_empty() {
              CPoll::Ready(Poll::Pending)
            } else {
              CPoll::Ready(Poll::Ready(Arc::new(buffered).into_boxed()))
            }
          }
          Poll::Pending => {
            if buffered.is_empty() {
              CPoll::Ready(Poll::Pending)
            } else {
              CPoll::Ready(Poll::Ready(Arc::new(buffered).into_boxed()))
            }
          }
        },
        CPoll::Blocked => {
          *self.buffered.write() = buffered;
          waker.clone().wake();
          CPoll::Blocked
        }
      }
    } else {
      *self.buffered.write() = buffered;
      self.waker.wake();
      CPoll::Blocked
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    if let Some(upstream) = self.upstream.try_read() {
      let view = upstream.access();
      if view.is_blocked() {
        return CPoll::Blocked;
      }
      let view = ForkedAccessView::<RwLockReadGuard<'static, Map>, K, V> {
        view: unsafe { std::mem::transmute(view.unwrap()) },
        lock: Arc::new(unsafe { std::mem::transmute(upstream) }),
      };
      CPoll::Ready(Box::new(view) as Box<dyn VirtualCollection<K, V>>)
    } else {
      CPoll::Blocked
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

/// notify all downstream proactively
struct BroadCast<K, V> {
  inner: Arc<RwLock<FastHashMap<u64, (Arc<AtomicWaker>, Sender<K, V>)>>>,
}

impl<K: CKey, V: CValue> futures::task::ArcWake for BroadCast<K, V> {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    let all = arc_self.inner.write();
    for v in all.values() {
      v.0.wake();
    }
  }
}

struct ForkedAccessView<T: 'static, K, V> {
  lock: Arc<RwLockReadGuard<'static, T>>,
  view: CollectionView<'static, K, V>,
}

impl<T: 'static, K, V> Clone for ForkedAccessView<T, K, V> {
  fn clone(&self) -> Self {
    Self {
      lock: self.lock.clone(),
      view: self.view.clone(),
    }
  }
}

impl<K: CKey, V: CValue, T: Send + Sync> VirtualCollection<K, V> for ForkedAccessView<T, K, V> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    self.view.iter_key_value()
  }

  fn access(&self, key: &K) -> Option<V> {
    self.view.access(key)
  }
}

impl<Map, K, V> ReactiveOneToManyRelationship<V, K> for ReactiveKVMapFork<Map, K, V>
where
  Map: ReactiveOneToManyRelationship<V, K>,
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CKey,
{
  fn multi_access(&self) -> CPoll<Box<dyn VirtualMultiCollection<V, K>>> {
    if let Some(upstream) = self.upstream.try_read() {
      let view = upstream.multi_access();
      if view.is_blocked() {
        return CPoll::Blocked;
      }
      let view = ForkedMultiAccessView::<RwLockReadGuard<'static, Map>, V, K> {
        view: unsafe { std::mem::transmute(view.unwrap()) },
        _lock: Arc::new(unsafe { std::mem::transmute(upstream) }),
      };
      CPoll::Ready(Box::new(view) as Box<dyn VirtualMultiCollection<V, K>>)
    } else {
      CPoll::Blocked
    }
  }
}

struct ForkedMultiAccessView<T: 'static, K, V> {
  _lock: Arc<RwLockReadGuard<'static, T>>,
  view: Box<dyn VirtualMultiCollection<K, V>>,
}

impl<K: CKey, V: CValue, T: Send + Sync> VirtualMultiCollection<K, V>
  for ForkedMultiAccessView<T, K, V>
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    self.view.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K, visitor: &mut dyn FnMut(V)) {
    self.view.access_multi(key, visitor)
  }
}
