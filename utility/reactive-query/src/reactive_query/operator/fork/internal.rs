use std::{panic::Location, sync::Weak};

use futures::channel::mpsc::*;

use super::helper::{finalize_buffered_changes, ForkedView};
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
  is_new_created: bool,
}

pub struct DownStreamFork<K, V> {
  rev: RwLock<Receiver<K, V>>,
  id: u64,
  pub as_static_forker: bool,
  waker: Arc<AtomicWaker>,
}

impl<K: CKey, V: CValue> DownStreamFork<K, V> {
  pub fn update_waker(&self, cx: &mut Context) {
    self.waker.register(cx.waker());
  }

  pub fn drain_changes(&self, cx: &mut Context) -> BoxedDynQuery<K, ValueChange<V>> {
    let mut buffered = Vec::new();
    while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(cx) {
      buffered.push(changes);
    }
    finalize_buffered_changes(buffered)
  }
}

pub struct ReactiveKVMapForkInternal<Map: ReactiveQuery> {
  upstream: Arc<RwLock<Map>>,
  downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<Map::Key, Map::Value>>>>,
  resolved: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
}

impl<Map: ReactiveQuery> Clone for ReactiveKVMapForkInternal<Map> {
  fn clone(&self) -> Self {
    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      resolved: self.resolved.clone(),
    }
  }
}

impl<Map: ReactiveQuery> ReactiveKVMapForkInternal<Map> {
  pub fn new(upstream: Map) -> Self {
    Self {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Default::default(),
      resolved: Default::default(),
    }
  }

  pub fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.write().request(request)
  }

  pub fn remove_child(&self, fork: &DownStreamFork<Map::Key, Map::Value>) {
    self.downstream.write().remove(&fork.id);
  }

  #[track_caller]
  pub fn create_child(&self, as_static_forker: bool) -> DownStreamFork<Map::Key, Map::Value> {
    let (sender, rev) = unbounded();
    let id = alloc_global_res_id();
    let waker: Arc<AtomicWaker> = Default::default();
    let info = DownStreamInfo {
      waker: waker.clone(),
      sender,
      should_send: !as_static_forker,
      create_location: *Location::caller(),
      is_new_created: true,
    };
    self.downstream.write().insert(id, info);

    // we early return the view if the upstream is already resolved, however when new child
    // created, the full view will be send and delta view. so this is required to reset
    *self.resolved.write() = None;

    DownStreamFork {
      rev: RwLock::new(rev),
      id,
      as_static_forker,
      waker,
    }
  }

  pub fn poll_and_broadcast(&self) -> ForkedView<<Map::Compute as QueryCompute>::View> {
    // if weak view cache exist, so that the view is not dropped and the upstream is locked
    // so that their can be any mutations in the upstream, so that we can directly return the view
    {
      let resolved = self.resolved.read();
      let resolved: &Option<Weak<dyn Any + Send + Sync>> = &resolved;
      if let Some(view) = resolved {
        if let Some(view) = view.upgrade() {
          return ForkedView {
            inner: view
              .downcast::<<Map::Compute as QueryCompute>::View>()
              .unwrap(),
          };
        }
      }
    }

    let (d, v) = self.poll_upstream_compute().resolve();
    let d = d.materialize();

    let mut downstream = self.downstream.write();
    for (_, info) in downstream.iter_mut() {
      if info.should_send {
        // when use full table as incremental table, we should only send full table.
        if info.is_new_created {
          let current = v
            .iter_key_value()
            .map(|(k, v)| (k, ValueChange::Delta(v, None)))
            .collect::<FastHashMap<_, _>>(); // todo, this should be shared if multiple downstream is new created
          let current = Arc::new(current);

          if !current.is_empty() {
            // we do not check unbound leak here because the sender is created here.
            info.sender.unbounded_send(current).ok();
          }
          info.is_new_created = false;
        } else {
          check_sender_has_leak(&info.sender, &info.create_location);
          info.sender.unbounded_send(d.clone()).ok();
        }
      }
    }

    // setup weak full view
    let v = Arc::new(v) as Arc<dyn Any + Send + Sync>;
    let view = v.clone();
    *self.resolved.write() = Some(Arc::downgrade(&v));

    ForkedView {
      inner: view
        .downcast::<<Map::Compute as QueryCompute>::View>()
        .unwrap(),
    }
  }

  fn poll_upstream_compute(&self) -> Map::Compute {
    /// a waker that notify all downstream proactively
    struct Broadcast<K, V> {
      inner: Arc<RwLock<FastHashMap<u64, DownStreamInfo<K, V>>>>,
    }

    impl<K: CKey, V: CValue> futures::task::ArcWake for Broadcast<K, V> {
      fn wake_by_ref(arc_self: &Arc<Self>) {
        let all = arc_self.inner.write();
        for v in all.values() {
          v.waker.wake();
        }
      }
    }

    let waker = Arc::new(Broadcast {
      inner: self.downstream.clone(),
    });
    let waker = futures::task::waker_ref(&waker);
    let mut cx = Context::from_waker(&waker);
    self.upstream.write().describe(&mut cx)
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
