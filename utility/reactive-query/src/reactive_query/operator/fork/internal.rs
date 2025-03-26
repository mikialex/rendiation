use std::{panic::Location, sync::Weak};

use futures::{channel::mpsc::*, task::noop_waker};

use super::helper::{finalize_buffered_changes, ForkedView};
use crate::*;

type ForkMessage<K, V> = Arc<FastHashMap<K, ValueChange<V>>>;

type Sender<K, V> = UnboundedSender<ForkMessage<K, V>>;
type Receiver<K, V> = UnboundedReceiver<ForkMessage<K, V>>;

pub struct DownStreamInfo<K, V> {
  waker: Arc<AtomicWaker>,
  sender: Sender<K, V>,
  /// some fork never receive message just act as a static forker, in this case the message should
  /// not send to it to avoid memory leak.
  should_send: bool,
  create_location: Location<'static>,
  is_new_created: bool,
}

pub struct DownStreamFork<K, V> {
  rev: Arc<RwLock<Receiver<K, V>>>,
  id: u64,
  pub as_static_forker: bool,
  waker: Arc<AtomicWaker>,
}

impl<K: CKey, V: CValue> DownStreamFork<K, V> {
  pub fn update_waker(&self, cx: &mut Context) {
    self.waker.register(cx.waker());
  }

  pub fn drain_changes(&self) -> DrainChange<K, V> {
    DrainChange {
      waker: self.waker.clone(),
      rev: self.rev.clone(),
    }
  }
}

#[derive(Clone)]
pub struct DrainChange<K, V> {
  waker: Arc<AtomicWaker>,
  rev: Arc<RwLock<Receiver<K, V>>>,
}

impl<K: CKey, V: CValue> DrainChange<K, V> {
  pub fn resolve(&mut self) -> BoxedDynQuery<K, ValueChange<V>> {
    // i think this is bad, why atomic waker not impl arc_wake?
    let waker = if let Some(waker) = self.waker.take() {
      self.waker.register(&waker);
      waker
    } else {
      noop_waker()
    };
    let mut cx = Context::from_waker(&waker);
    let mut buffered = Vec::new();
    while let Poll::Ready(Some(changes)) = self.rev.write().poll_next_unpin(&mut cx) {
      buffered.push(changes);
    }
    finalize_buffered_changes(buffered)
  }
}

pub struct ReactiveKVMapForkInternal<Map, K, V> {
  upstream: Arc<RwLock<Map>>,
  pub(super) downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<K, V>>>>,
  described: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
  resolved: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
  future_forker: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
}

impl<Map: ReactiveQuery> Clone for ReactiveKVMapForkInternal<Map, Map::Key, Map::Value> {
  fn clone(&self) -> Self {
    Self {
      upstream: self.upstream.clone(),
      downstream: self.downstream.clone(),
      resolved: self.resolved.clone(),
      described: self.described.clone(),
      future_forker: self.future_forker.clone(),
    }
  }
}

impl<Map, K, V> ReactiveKVMapForkInternal<Map, K, V> {
  pub fn new(upstream: Map) -> Self {
    Self {
      upstream: Arc::new(RwLock::new(upstream)),
      downstream: Default::default(),
      resolved: Default::default(),
      future_forker: Default::default(),
      described: Default::default(),
    }
  }

  pub fn remove_child(&self, fork: &DownStreamFork<K, V>) {
    self.downstream.write().remove(&fork.id);
  }
}

impl<Map: ReactiveQuery> ReactiveKVMapForkInternal<Map, Map::Key, Map::Value> {
  pub fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.write().request(request)
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
    *self.described.write() = None;

    DownStreamFork {
      rev: Arc::new(RwLock::new(rev)),
      id,
      as_static_forker,
      waker,
    }
  }

  pub fn describe_view(&self) -> ForkComputeView<Map::Compute> {
    // if weak view cache exist, so the view is not dropped and the upstream is locked
    // so their can be any mutations in the upstream, so we can directly return the view

    let mut already_resolved_view = None;
    {
      let resolved = self.resolved.read();
      let resolved: &Option<Weak<dyn Any + Send + Sync>> = &resolved;
      if let Some(view) = resolved {
        if let Some(view) = view.upgrade() {
          already_resolved_view = view
            .downcast::<<Map::Compute as QueryCompute>::View>()
            .unwrap()
            .into();
        }
      }
    }

    // this optimization is not valid for now, todo, fix any implementation that drop the full view.
    // tip, search "let (d, _) ="
    // once view dropped, the compute will re resolve. but if we enable compute desc reuse(down here)
    // the compute will resolve same message again.
    // {
    //   let described = self.described.read();
    //   let described: &Option<Weak<dyn Any + Send + Sync>> = &described;
    //   if let Some(compute) = described {
    //     if let Some(compute) = compute.upgrade() {
    //       return ForkComputeView::ViewToCompute {
    //         compute: compute.downcast::<RwLock<Map::Compute>>().unwrap(),
    //         downstream: self.downstream.clone(),
    //         view_resolve: self.resolved.clone(),
    //       };
    //     }
    //   }
    // }

    // do not create compute instance when view is ready because the implementation of the
    // compute might contains exclusive lock access, which cause dead lock.
    let compute = if already_resolved_view.is_none() {
      self.describe_upstream_view().into()
    } else {
      None
    };

    ForkComputeView {
      _already_resolved_view: already_resolved_view,
      compute,
      downstream: self.downstream.clone(),
      view_resolve: self.resolved.clone(),
      future_forker: self.future_forker.clone(),
    }
  }

  fn describe_upstream_view(&self) -> Arc<RwLock<Map::Compute>> {
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
    let d = Arc::new(RwLock::new(self.upstream.write().describe(&mut cx)));
    let rd = d.clone();

    let d = d as Arc<dyn Any + Send + Sync>;
    *self.described.write() = Some(Arc::downgrade(&d));

    rd
  }
}

pub struct ForkComputeView<T: QueryCompute> {
  // we must keep resolved view here because the view might be dropped
  pub(super) _already_resolved_view: Option<Arc<T::View>>,
  pub(super) compute: Option<Arc<RwLock<T>>>,
  pub(super) downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<T::Key, T::Value>>>>,
  pub(super) view_resolve: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
  #[allow(dead_code)]
  pub(super) future_forker: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
}

impl<Map: QueryCompute> ForkComputeView<Map> {
  pub fn resolve(&mut self) -> ForkedView<Map::View> {
    // early return if view has already computed
    {
      let resolved = self.view_resolve.read();
      let resolved: &Option<Weak<dyn Any + Send + Sync>> = &resolved;
      if let Some(view) = resolved {
        if let Some(view) = view.upgrade() {
          return ForkedView {
            inner: view.downcast::<Map::View>().unwrap(),
          };
        }
      }
    }

    let (d, v) = self.compute.as_ref().unwrap().write().resolve();
    let d = d.materialize();

    let mut downstream = self.downstream.write();
    for (_, info) in downstream.iter_mut() {
      if info.should_send {
        // when use full table as incremental table, we should only send full table.
        if info.is_new_created {
          // todo, this compute should be shared if multiple downstream is new created
          let current = v
            .iter_key_value()
            .map(|(k, v)| (k, ValueChange::Delta(v, None)))
            .collect::<FastHashMap<_, _>>();
          let current = Arc::new(current);

          if !current.is_empty() {
            // we do not check unbound leak here because the sender is created here.
            info.sender.unbounded_send(current).ok();
          }
          info.is_new_created = false;
        } else if !d.is_empty() {
          check_sender_has_leak(&info.sender, &info.create_location);
          info.sender.unbounded_send(d.clone()).ok();
        }
      }
    }

    // setup weak full view
    let arc_view = Arc::new(v);
    self._already_resolved_view = Some(arc_view.clone());
    let v = arc_view.clone() as Arc<dyn Any + Send + Sync>;
    *self.view_resolve.write() = Some(Arc::downgrade(&v));

    ForkedView {
      inner: arc_view.clone(),
    }
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
