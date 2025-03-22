use std::{panic::Location, sync::Weak};

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
  create_location: Location<'static>,
}

pub struct ReactiveKVMapFork<Map: ReactiveQuery> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<Map::Key, Map::Value>>>>,
  resolved: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
  buffered: RwLock<Vec<ForkMessage<Map::Key, Map::Value>>>,
  rev: RwLock<Receiver<Map::Key, Map::Value>>,
  id: u64,
  as_static_forker: bool,
  waker: Arc<AtomicWaker>,
}

impl<Map: ReactiveQuery> ReactiveKVMapFork<Map> {
  #[track_caller]
  pub fn new(upstream: Map, as_static_forker: bool) -> Self {
    let (sender, rev) = unbounded();
    let mut init = FastHashMap::default();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    let info = DownStreamInfo {
      waker: waker.clone(),
      sender,
      should_send: !as_static_forker,
      create_location: *Location::caller(),
    };
    init.insert(id, info);
    ReactiveKVMapFork {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Arc::new(RwLock::new(init)),
      resolved: Default::default(),
      rev: RwLock::new(rev),
      id,
      waker,
      as_static_forker,
      buffered: Default::default(),
    }
  }
}

impl<K: CKey, V: CValue> RQForker<K, V> {
  pub fn update_and_read(&self) -> BoxedDynQuery<K, V> {
    assert!(self.as_static_forker);
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx = Context::from_waker(&waker);
    let (_, v) = self.upstream.read().describe(&mut cx).resolve();
    Box::new(v)
  }
}

impl<K: CKey, V: CKey> OneManyRelationForker<K, V> {
  pub fn update_and_read(&self) -> BoxedDynMultiQuery<K, V> {
    assert!(self.as_static_forker);
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx = Context::from_waker(&waker);
    let (_, _, v) = self.upstream.read().poll_changes_with_inv_dyn(&mut cx);
    v
  }
}

impl<Map: ReactiveQuery> Drop for ReactiveKVMapFork<Map> {
  fn drop(&mut self) {
    self.downstream.write().remove(&self.id);
  }
}

impl<Map> Clone for ReactiveKVMapFork<Map>
where
  Map: ReactiveQuery,
{
  fn clone(&self) -> Self {
    self.clone_impl(false)
  }
}

const ENABLE_MESSAGE_LEAK_ASSERT: bool = false;
fn check_sender_has_leak<T>(sender: &UnboundedSender<T>, loc: &Location<'static>) {
  if ENABLE_MESSAGE_LEAK_ASSERT && sender.len() > 50 {
    panic!(
      "potential reactive query fork message leak:,\n current sender buffered message count: {},\n create_location: {}",
      sender.len(),
      loc,
    );
  }
}

impl<Map> ReactiveKVMapFork<Map>
where
  Map: ReactiveQuery,
{
  #[track_caller]
  pub fn clone_as_static(&self) -> Self {
    self.clone_impl(true)
  }

  #[track_caller]
  fn clone_impl(&self, as_static_forker: bool) -> Self {
    // get updated current view and delta; this is necessary, because we require
    // current view as init delta for new forked downstream
    //
    // update should use downstream registered waker
    let waker = Arc::new(BroadCast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx = Context::from_waker(&waker);
    let (d, v) = self.upstream.read().describe(&mut cx).resolve();

    // delta should also dispatch to downstream to keep message integrity
    let downstream = self.downstream.read_recursive();
    let d = d.materialize();
    for (_, info) in downstream.iter() {
      if info.should_send {
        check_sender_has_leak(&info.sender, &info.create_location);
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
      // we do not check unbound leak here because the sender is created here.
      sender.unbounded_send(current).ok();
    }

    let waker: Arc<AtomicWaker> = Default::default();
    let info = DownStreamInfo {
      waker: waker.clone(),
      sender,
      should_send: !as_static_forker,
      create_location: *Location::caller(),
    };

    downstream.insert(id, info);

    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      id,
      rev: RwLock::new(rev),
      resolved: self.resolved.clone(),
      waker,
      buffered: Default::default(),
      as_static_forker,
    }
  }
}

fn finalize_buffered_changes<K: CKey, V: CValue>(
  mut changes: Vec<ForkMessage<K, V>>,
) -> BoxedDynQuery<K, ValueChange<V>> {
  if changes.is_empty() {
    return Box::new(EmptyQuery::default());
  }

  if changes.len() == 1 {
    let first = changes.pop().unwrap();
    if first.is_empty() {
      return Box::new(EmptyQuery::default());
    } else {
      return Box::new(first);
    }
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(&mut target, c.iter().map(|(k, v)| (k.clone(), v.clone())));
  }

  if target.is_empty() {
    Box::new(EmptyQuery::default())
  } else {
    Box::new(target)
  }
}

#[derive(Clone)]
pub struct ForkedView<T> {
  inner: Arc<T>,
}

impl<T: Query> Query for ForkedView<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.inner.iter_key_value()
  }

  fn access(&self, key: &T::Key) -> Option<T::Value> {
    self.inner.access(key)
  }
}
impl<T: MultiQuery> MultiQuery for ForkedView<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_keys(&self) -> impl Iterator<Item = T::Key> + '_ {
    self.inner.iter_keys()
  }

  fn access_multi(&self, key: &T::Key) -> Option<impl Iterator<Item = T::Value> + '_> {
    self.inner.access_multi(key)
  }
}

impl<Map> ReactiveQuery for ReactiveKVMapFork<Map>
where
  Map: ReactiveQuery,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = (
    impl Query<Key = Self::Key, Value = ValueChange<Self::Value>>,
    ForkedView<<Map::Compute as QueryCompute>::View>,
  );
  fn describe(&self, cx: &mut Context) -> Self::Compute {
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
            inner: view
              .downcast::<<Map::Compute as QueryCompute>::View>()
              .unwrap(),
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
    let (d, v) = upstream.describe(&mut cx_2).resolve();

    let d = {
      let downstream = self.downstream.write();
      let d = d.materialize();
      if !d.is_empty() {
        // broad cast to others
        // we are not required to call broadcast waker because it will be waked by others
        // receivers
        for (id, downstream) in downstream.iter() {
          if *id != self.id && downstream.should_send {
            check_sender_has_leak(&downstream.sender, &downstream.create_location);
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
      inner: view
        .downcast::<<Map::Compute as QueryCompute>::View>()
        .unwrap(),
    };

    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.write().request(request)
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
