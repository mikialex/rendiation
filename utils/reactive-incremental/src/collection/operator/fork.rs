use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::FastHashMap;
use futures::task::AtomicWaker;
use parking_lot::RwLockReadGuard;

use crate::*;

type Sender<K, V> =
  futures::channel::mpsc::UnboundedSender<Arc<FastHashMap<K, CollectionDelta<K, V>>>>;
type Receiver<K, V> =
  futures::channel::mpsc::UnboundedReceiver<Arc<FastHashMap<K, CollectionDelta<K, V>>>>;

pub type ReactiveKVMapFork<Map, K, V> = BufferedCollection<ReactiveKVMapForkImpl<Map, K, V>, K, V>;

pub struct ReactiveKVMapForkImpl<Map, K, V> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, (Arc<AtomicWaker>, Sender<K, V>)>>>,
  rev: RwLock<Receiver<K, V>>,
  id: u64,
  waker: Arc<AtomicWaker>,
  phantom: PhantomData<(K, V)>,
}

impl<Map, K, V> ReactiveKVMapForkImpl<Map, K, V> {
  pub fn new(upstream: Map) -> Self {
    let (sender, rev) = futures::channel::mpsc::unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    init.insert(id, (waker.clone(), sender));
    ReactiveKVMapForkImpl {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Arc::new(RwLock::new(init)),
      rev: RwLock::new(rev),
      id,
      waker,
      phantom: Default::default(),
    }
  }
}

impl<Map, K, V> Drop for ReactiveKVMapForkImpl<Map, K, V> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}

impl<K, V, Map> Clone for ReactiveKVMapForkImpl<Map, K, V>
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
      .map(|(k, v)| (k.clone(), CollectionDelta::Delta(k, v, None)))
      .collect::<FastHashMap<_, _>>();
    let current = Arc::new(current);

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    let (sender, rev) = futures::channel::mpsc::unbounded();

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
    }
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapForkImpl<Map, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V> {
    if self.waker.take().is_some() {
      self.waker.register(cx.waker());
      // the previous waker not waked, nothing changes, return
      return CPoll::Ready(Poll::Pending);
    }
    self.waker.register(cx.waker());

    let r = self.rev.write().poll_next_unpin(cx);
    if r.is_ready() {
      self.waker.wake();
      return match r {
        Poll::Ready(Some(v)) => CPoll::Ready(Poll::Ready(Box::new(v))),
        _ => unreachable!(),
      };
    }

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
            let c = v.materialize(); // todo improve
            for downstream in downstream.values() {
              downstream.1.unbounded_send(c.clone()).ok();
            }
            drop(downstream);
            waker.clone().wake();
            match self.rev.write().poll_next_unpin(cx) {
              Poll::Ready(Some(v)) => CPoll::Ready(Poll::Ready(Box::new(v))),
              _ => unreachable!(),
            }
          }
          Poll::Pending => CPoll::Ready(Poll::Pending),
        },
        CPoll::Blocked => {
          waker.clone().wake();
          CPoll::Blocked
        }
      }
    } else {
      self.waker.wake();
      CPoll::Blocked
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    if let Some(upstream) = self.upstream.try_read() {
      let view = upstream.access();
      let view = ForkedAccessView::<RwLockReadGuard<'static, Map>, K, V> {
        view: unsafe { std::mem::transmute(view) },
        lock: unsafe { std::mem::transmute(upstream) },
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

// impl<K, V, T, Map> VirtualMultiCollection<K, V> for ReactiveKVMapForkImpl<Map, T, V, K>
// where
//   Map: VirtualMultiCollection<K, V> + Sync + Send,
// {
//   fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
//     // todo avoid clone
//     self
//       .upstream
//       .read()
//       .iter_key_in_multi_collection()
//       .collect::<Vec<_>>()
//       .into_iter()
//   }

//   // todo remove box
//   fn access_multi(&self) -> impl Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_ {
//     let inner = self.upstream.read();
//     let acc = inner.access_multi();

//     let acc = Box::new(acc) as Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>;

//     // safety: read guard is hold by closure, acc's real reference is form the Map
//     let acc: Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_> =
//       unsafe { std::mem::transmute(acc) };

//     move |key: &_, f| {
//       let _holder = &inner;
//       let acc = &acc;
//       acc(key, f)
//     }
//   }

//   // todo remove box
//   fn try_access_multi(&self) -> Option<Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>> {
//     let inner = self.upstream.try_read()?;
//     let acc = inner.try_access_multi()?;

//     // safety: read guard is hold by closure, acc's real reference is form the Map
//     let acc: Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_> =
//       unsafe { std::mem::transmute(acc) };

//     let acc = move |key: &_, f: &mut dyn FnMut(V)| {
//       let _holder = &inner;
//       let acc = &acc;
//       acc(key, f)
//     };

//     let boxed = Box::new(acc) as Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>;
//     boxed.into()
//   }
// }
