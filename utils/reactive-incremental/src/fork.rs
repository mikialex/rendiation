use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::FastHashMap;
use futures::task::AtomicWaker;
use parking_lot::{RwLock, RwLockReadGuard};

use crate::*;

type Sender<T> = futures::channel::mpsc::UnboundedSender<T>;
type Receiver<T> = futures::channel::mpsc::UnboundedReceiver<T>;

pub type ReactiveKVMapFork<Map, T, K, V> =
  BufferedCollection<T, ReactiveKVMapForkImpl<Map, T, K, V>>;

pub struct ReactiveKVMapForkImpl<Map, T, K, V> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, (Arc<AtomicWaker>, Sender<T>)>>>,
  rev: Receiver<T>,
  id: u64,
  waker: Arc<AtomicWaker>,
  phantom: PhantomData<(K, V)>,
}

impl<Map, T, K, V> ReactiveKVMapForkImpl<Map, T, K, V> {
  pub fn new(upstream: Map) -> Self {
    let (sender, rev) = futures::channel::mpsc::unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    init.insert(id, (waker.clone(), sender));
    ReactiveKVMapForkImpl {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Arc::new(RwLock::new(init)),
      rev,
      id,
      waker,
      phantom: Default::default(),
    }
  }
}

impl<Map, T, K, V> Drop for ReactiveKVMapForkImpl<Map, T, K, V> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}

trait RebuildTable<K, V>: Sized {
  // None is empty case
  fn from_table(c: &impl VirtualCollection<K, V>) -> Option<Self>;
}

impl<K: Clone + Eq + std::hash::Hash, V> RebuildTable<K, V> for CollectionChanges<K, V> {
  fn from_table(c: &impl VirtualCollection<K, V>) -> Option<Self> {
    let c = c
      .iter_key_value_forgive()
      .map(|(k, v)| (k.clone(), CollectionDelta::Delta(k, v)))
      .collect::<Self>();
    (!c.is_empty()).then_some(c)
  }
}

impl<K: Clone + Eq + std::hash::Hash, V> RebuildTable<K, V>
  for CollectionChangesWithPrevious<K, V>
{
  fn from_table(c: &impl VirtualCollection<K, V>) -> Option<Self> {
    let c = c
      .iter_key_value_forgive()
      .map(|(k, v)| (k.clone(), CollectionDeltaWithPrevious::Delta(k, v, None)))
      .collect::<Self>();
    (!c.is_empty()).then_some(c)
  }
}

impl<K, V, Map: VirtualCollection<K, V>, T: RebuildTable<K, V>> Clone
  for ReactiveKVMapForkImpl<Map, T, K, V>
{
  fn clone(&self) -> Self {
    // when fork the collection, we should pass the current table as the init change
    let upstream = self.upstream.read_recursive();

    let u: &Map = &upstream;
    let current = T::from_table(u);

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    let (sender, rev) = futures::channel::mpsc::unbounded();

    if let Some(current) = current {
      sender.unbounded_send(current).ok();
    }

    let waker: Arc<AtomicWaker> = Default::default();
    downstream.insert(id, (waker.clone(), sender));

    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      id,
      rev,
      waker,
      phantom: PhantomData,
    }
  }
}

impl<Map, K, V> ReactiveCollection<K, V>
  for ReactiveKVMapForkImpl<Map, CollectionChanges<K, V>, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<K, V>> {
    if self.waker.take().is_some() {
      self.waker.register(cx.waker());
      // the previous waker not waked, nothing changes, return
      return CPoll::Pending;
    }
    self.waker.register(cx.waker());

    let r = self.rev.poll_next_unpin(cx);
    if r.is_ready() {
      return match r {
        Poll::Ready(Some(v)) => CPoll::Ready(v),
        _ => CPoll::Pending,
      };
    }

    if let Some(mut upstream) = self.upstream.try_write() {
      let waker = Arc::new(BroadCast {
        inner: self.downstream.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx_2 = Context::from_waker(&waker);
      let r = upstream.poll_changes(&mut cx_2);

      if let CPoll::Ready(v) = r {
        let downstream = self.downstream.write();
        for downstream in downstream.values() {
          downstream.1.unbounded_send(v.clone()).ok();
        }
      } else {
        return r;
      }
    } else {
      return CPoll::Blocked;
    }
    self.poll_changes(cx)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

/// notify all downstream proactively
struct BroadCast<T> {
  inner: Arc<RwLock<FastHashMap<u64, (Arc<AtomicWaker>, Sender<T>)>>>,
}

impl<T: Send + Sync + Clone> futures::task::ArcWake for BroadCast<T> {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    let all = arc_self.inner.write();
    for v in all.values() {
      v.0.wake();
    }
  }
}

impl<Map, K, V> ReactiveCollectionWithPrevious<K, V>
  for ReactiveKVMapForkImpl<Map, CollectionChangesWithPrevious<K, V>, K, V>
where
  Map: ReactiveCollectionWithPrevious<K, V>,
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChangesWithPrevious<K, V>> {
    if self.waker.take().is_some() {
      self.waker.register(cx.waker());
      // the previous waker not waked, nothing changes, return
      return CPoll::Pending;
    }
    self.waker.register(cx.waker());

    let r = self.rev.poll_next_unpin(cx);
    if r.is_ready() {
      return match r {
        Poll::Ready(Some(v)) => CPoll::Ready(v),
        _ => CPoll::Pending,
      };
    }

    if let Some(mut upstream) = self.upstream.try_write() {
      let waker = Arc::new(BroadCast {
        inner: self.downstream.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx_2 = Context::from_waker(&waker);
      let r = upstream.poll_changes(&mut cx_2);

      if let CPoll::Ready(v) = r {
        let downstream = self.downstream.write();
        for downstream in downstream.values() {
          downstream.1.unbounded_send(v.clone()).ok();
        }
      } else {
        return r;
      }
    } else {
      return CPoll::Blocked;
    }
    self.poll_changes(cx)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

impl<K, V, T, Map> VirtualCollection<K, V> for ReactiveKVMapForkImpl<Map, T, K, V>
where
  Map: VirtualCollection<K, V> + 'static + Sync,
  K: Send,
  V: Send,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    struct ReactiveKVMapForkRead<'a, Map, I> {
      _inner: RwLockReadGuard<'a, Map>,
      inner_iter: I,
    }

    impl<'a, Map, I: Iterator> Iterator for ReactiveKVMapForkRead<'a, Map, I> {
      type Item = I::Item;

      fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
      }
    }

    /// util to get collection's accessor type
    type IterOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Iterator<Item = K> + 'a;
    fn get_iter<'a, K, V, M>(map: &M) -> IterOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.iter_key()
    }

    let inner = self.upstream.read();
    let inner_iter = get_iter(inner.deref());
    // safety: read guard is hold by iter, acc's real reference is form the Map
    let inner_iter: IterOf<'static, Map, K, V> = unsafe { std::mem::transmute(inner_iter) };
    ReactiveKVMapForkRead {
      _inner: inner,
      inner_iter,
    }
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.upstream.read();

    /// util to get collection's accessor type
    type AccessorOf<'a, M: VirtualCollection<K, V> + 'a, K, V> = impl Fn(&K) -> Option<V> + 'a;
    fn get_accessor<'a, K, V, M>(map: &M) -> AccessorOf<M, K, V>
    where
      M: VirtualCollection<K, V> + 'a,
    {
      map.access()
    }

    let acc: AccessorOf<Map, K, V> = get_accessor(inner.deref());
    // safety: read guard is hold by closure, acc's real reference is form the Map
    let acc: AccessorOf<'static, Map, K, V> = unsafe { std::mem::transmute(acc) };
    move |key| {
      let _holder = &inner;
      let acc = &acc;
      acc(key)
    }
  }

  fn try_access(&self) -> Option<Box<dyn Fn(&K) -> Option<V> + Sync + '_>> {
    let inner = self.upstream.try_read()?;
    let acc = inner.try_access()?;

    // safety: read guard is hold by closure, acc's real reference is form the Map
    let acc: Box<dyn Fn(&K) -> Option<V> + Sync + '_> = unsafe { std::mem::transmute(acc) };

    let acc = move |key: &_| {
      let _holder = &inner;
      let acc = &acc;
      acc(key)
    };

    let boxed = Box::new(acc) as Box<dyn Fn(&K) -> Option<V> + Sync + '_>;
    boxed.into()
  }
}

impl<K, V, T, Map> VirtualMultiCollection<K, V> for ReactiveKVMapForkImpl<Map, T, V, K>
where
  Map: VirtualMultiCollection<K, V> + Sync + Send,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    // todo avoid clone
    self
      .upstream
      .read()
      .iter_key_in_multi_collection()
      .collect::<Vec<_>>()
      .into_iter()
  }

  // todo remove box
  fn access_multi(&self) -> impl Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_ {
    let inner = self.upstream.read();
    let acc = inner.access_multi();

    let acc = Box::new(acc) as Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>;

    // safety: read guard is hold by closure, acc's real reference is form the Map
    let acc: Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_> =
      unsafe { std::mem::transmute(acc) };

    move |key: &_, f| {
      let _holder = &inner;
      let acc = &acc;
      acc(key, f)
    }
  }

  // todo remove box
  fn try_access_multi(&self) -> Option<Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>> {
    let inner = self.upstream.try_read()?;
    let acc = inner.try_access_multi()?;

    // safety: read guard is hold by closure, acc's real reference is form the Map
    let acc: Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_> =
      unsafe { std::mem::transmute(acc) };

    let acc = move |key: &_, f: &mut dyn FnMut(V)| {
      let _holder = &inner;
      let acc = &acc;
      acc(key, f)
    };

    let boxed = Box::new(acc) as Box<dyn Fn(&K, &mut dyn FnMut(V)) + Send + Sync + '_>;
    boxed.into()
  }
}
