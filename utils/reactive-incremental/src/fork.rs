use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::FastHashMap;
use parking_lot::{RwLock, RwLockReadGuard};

use crate::*;

type Sender<T> = futures::channel::mpsc::UnboundedSender<T>;
type Receiver<T> = futures::channel::mpsc::UnboundedReceiver<T>;

pub struct ReactiveKVMapFork<Map, T, K, V> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, Sender<FastPassingVec<T>>>>>,
  rev: Receiver<FastPassingVec<T>>,
  id: u64,
  phantom: PhantomData<(K, V)>,
}

impl<Map, T, K, V> ReactiveKVMapFork<Map, T, K, V> {
  pub fn new(upstream: Map) -> Self {
    let (sender, rev) = futures::channel::mpsc::unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    init.insert(id, sender);
    ReactiveKVMapFork {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Arc::new(RwLock::new(init)),
      rev,
      id,
      phantom: Default::default(),
    }
  }
}

impl<Map, T, K, V> Drop for ReactiveKVMapFork<Map, T, K, V> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}
impl<K, V, Map: VirtualCollection<K, V>> Clone
  for ReactiveKVMapFork<Map, CollectionDelta<K, V>, K, V>
{
  fn clone(&self) -> Self {
    // when fork the collection, we should pass the current table as the init change
    let upstream = self.upstream.read_recursive();
    let keys = upstream.iter_key();
    let access = upstream.access();
    // todo, currently we not enforce that the access should be match the iter_key result so
    // required to handle None case
    let deltas = keys
      .filter_map(|key| access(&key).map(|v| (key, v)))
      .map(|(k, v)| CollectionDelta::Delta(k, v))
      .collect::<Vec<_>>();

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    // we don't expect clone in real runtime so we don't care about wake
    let (sender, rev) = futures::channel::mpsc::unbounded();

    if !deltas.is_empty() {
      let deltas = FastPassingVec::from_vec(deltas);
      sender.unbounded_send(deltas).ok();
    }

    downstream.insert(id, sender);

    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      id,
      rev,
      phantom: PhantomData,
    }
  }
}

impl<K, V, Map: VirtualCollection<K, V>> Clone
  for ReactiveKVMapFork<Map, CollectionDeltaWithPrevious<K, V>, K, V>
{
  fn clone(&self) -> Self {
    // when fork the collection, we should pass the current table as the init change
    let upstream = self.upstream.read_recursive();
    let keys = upstream.iter_key();
    let access = upstream.access();
    // todo, currently we not enforce that the access should be match the iter_key result so
    // required to handle None case
    let deltas = keys
      .filter_map(|key| access(&key).map(|v| (key, v)))
      .map(|(k, v)| CollectionDeltaWithPrevious::Delta(k, v, None))
      .collect::<Vec<_>>();

    let mut downstream = self.downstream.write();
    let id = alloc_global_res_id();
    // we don't expect clone in real runtime so we don't care about wake
    let (sender, rev) = futures::channel::mpsc::unbounded();

    if !deltas.is_empty() {
      let deltas = FastPassingVec::from_vec(deltas);
      sender.unbounded_send(deltas).ok();
    }

    downstream.insert(id, sender);

    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      id,
      rev,
      phantom: PhantomData,
    }
  }
}

impl<Map, K, V> ReactiveCollection<K, V> for ReactiveKVMapFork<Map, CollectionDelta<K, V>, K, V>
where
  Map: ReactiveCollection<K, V>,
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = FastPassingVec<CollectionDelta<K, V>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    // these writes should not deadlock, because we not prefer the concurrency between the table
    // updates. if we do allow it in the future, just change it to try write or yield pending.

    let r = self.rev.poll_next_unpin(cx);
    if r.is_ready() {
      return r;
    }

    let mut upstream = self.upstream.write();
    let r = upstream.poll_changes(cx);

    if let Poll::Ready(Some(v)) = r {
      let downstream = self.downstream.write();
      let vec = v.collect_into_pass_vec();
      for downstream in downstream.values() {
        downstream.unbounded_send(vec.clone()).ok();
      }
    } else {
      return Poll::Pending;
    }
    drop(upstream);

    self.poll_changes(cx)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

impl<Map, K, V> ReactiveCollectionWithPrevious<K, V>
  for ReactiveKVMapFork<Map, CollectionDeltaWithPrevious<K, V>, K, V>
where
  Map: ReactiveCollectionWithPrevious<K, V>,
  K: Clone + Send + Sync + 'static,
  V: Clone + Send + Sync + 'static,
{
  type Changes = FastPassingVec<CollectionDeltaWithPrevious<K, V>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    // these writes should not deadlock, because we not prefer the concurrency between the table
    // updates. if we do allow it in the future, just change it to try write or yield pending.

    let r = self.rev.poll_next_unpin(cx);
    if r.is_ready() {
      return r;
    }

    let mut upstream = self.upstream.write();
    let r = upstream.poll_changes(cx);

    if let Poll::Ready(Some(v)) = r {
      let downstream = self.downstream.write();
      let vec = v.collect_into_pass_vec();
      for downstream in downstream.values() {
        downstream.unbounded_send(vec.clone()).ok();
      }
      // }
    } else {
      return Poll::Pending;
    }
    drop(upstream);

    self.poll_changes(cx)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.write().extra_request(request)
  }
}

impl<K, V, T, Map> VirtualCollection<K, V> for ReactiveKVMapFork<Map, T, K, V>
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
}
