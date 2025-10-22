use std::any::Any;
use std::any::TypeId;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;

use fast_hash_collection::*;
use futures::FutureExt;
use futures::stream::*;
use parking_lot::RwLock;
pub use query::*;

mod task_pool;
mod use_result;

pub use hook::*;
pub use task_pool::*;
pub use use_result::*;

#[derive(Default)]
pub struct ChangeCollector {
  scope: FastHashMap<u32, bool>,
  changed: bool,
}

impl ChangeCollector {
  #[inline(never)]
  pub fn notify_change(&mut self) {
    self.scope.values_mut().for_each(|v| *v = true);
    self.changed = true;
  }
}

pub enum QueryHookStage<'a> {
  SpawnTask {
    spawner: &'a TaskSpawner,
    pool: &'a mut AsyncTaskPool,
    change_collector: &'a mut ChangeCollector,
    ctx: &'a mut Context<'a>,
  },
  ResolveTask {
    task: &'a mut TaskPoolResultCx,
  },
  Other,
}

pub enum TaskUseResult<T> {
  SpawnId(u32),
  Result(T),
  NotInStage,
}

impl<T: Clone + 'static> TaskUseResult<T> {
  pub fn into_use_result(self, cx: &mut impl QueryHookCxLike) -> UseResult<T> {
    match self {
      TaskUseResult::SpawnId(id) => {
        if let QueryHookStage::SpawnTask { pool, .. } = cx.stage() {
          UseResult::SpawnStageFuture(pool.share_task_by_id(id))
        } else {
          unreachable!()
        }
      }
      TaskUseResult::Result(result) => UseResult::ResolveStageReady(result),
      TaskUseResult::NotInStage => UseResult::NotInStage,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShareKey {
  TypeId(TypeId),
  Hash(u64),
}

/// this trait serves two purposes:
/// - workaround/express a lifetime issue without unsafe
/// - support custom shared
pub trait SharedResultProvider<Cx>: 'static {
  type Result: Clone + Sync + Send + 'static;
  fn compute_share_key(&self) -> ShareKey {
    ShareKey::TypeId(TypeId::of::<Self>())
  }
  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result>;
}

pub type SharedHashMap<K, V> = Arc<RwLock<FastHashMap<K, V>>>;
pub type SharedHashMapRead<K, V> = LockReadGuardHolder<FastHashMap<K, V>>;
#[inline(always)]
pub fn maintain_shared_map<K: CKey, V, D: DataChanges<Key = K>>(
  map: &SharedHashMap<K, V>,
  change: UseResult<D>,
  f: impl FnMut(D::Value) -> V,
) {
  maintain_shared_map_avoid_unnecessary_creator_init(map, change, || f)
}

pub fn maintain_shared_map_avoid_unnecessary_creator_init<K, V, D, F>(
  map: &SharedHashMap<K, V>,
  change: UseResult<D>,
  f: impl FnOnce() -> F,
) where
  K: CKey,
  D: DataChanges<Key = K>,
  F: FnMut(D::Value) -> V,
{
  if let Some(changes) = change.if_ready()
    && changes.has_change()
  {
    let mut f = f();
    let mut map = map.write();
    for k in changes.iter_removed() {
      map.remove(&k);
    }
    for (k, v) in changes.iter_update_or_insert() {
      map.insert(k, f(v));
    }
  }
}

pub trait QueryHookCxLike: HooksCxLike {
  fn is_spawning_stage(&self) -> bool;
  fn is_resolve_stage(&self) -> bool;
  fn stage(&mut self) -> QueryHookStage<'_>;

  fn spawner(&mut self) -> Option<TaskSpawner> {
    if let QueryHookStage::SpawnTask { spawner, .. } = self.stage() {
      Some(spawner.clone())
    } else {
      None
    }
  }

  fn when_spawning_stage(&self, f: impl FnOnce()) {
    if self.is_spawning_stage() {
      f();
    }
  }

  fn when_resolve_stage<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
    if self.is_resolve_stage() {
      f().into()
    } else {
      None
    }
  }

  // maybe this fn should move to upstream
  fn use_shared_hash_map<K: 'static, V: 'static>(&mut self) -> SharedHashMap<K, V> {
    let (_, r) = self.use_plain_state_default_cloned::<SharedHashMap<K, V>>();
    r
  }

  fn use_begin_change_set_collect(
    &mut self,
  ) -> (&mut Self, impl FnOnce(&mut Self) -> Option<bool>) {
    use std::sync::atomic::AtomicU32;
    static ID: AtomicU32 = AtomicU32::new(0);
    fn get_new_id() -> u32 {
      ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    let (cx, id) = self.use_plain_state::<u32>(get_new_id);

    if let QueryHookStage::SpawnTask {
      change_collector, ..
    } = cx.stage()
    {
      change_collector.scope.insert(*id, false);
    }

    (cx, move |cx| {
      if let QueryHookStage::SpawnTask {
        change_collector, ..
      } = cx.stage()
      {
        let has_change = change_collector.scope.remove(id).unwrap();
        Some(has_change)
      } else {
        None
      }
    })
  }

  /// when f contains future and is spawning stage, the future will be spawned
  /// and the return result contains task id in spawn stage or result in resolve stage
  fn use_global_shared_future<R: 'static + Send + Sync + Clone>(
    &mut self,
    f: Option<impl Future<Output = R> + Send + Sync + 'static>,
  ) -> TaskUseResult<R> {
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { pool, .. } => {
        if let Some(fut) = f {
          *token = pool.install_task(fut);
          TaskUseResult::SpawnId(*token)
        } else {
          *token = u32::MAX;
          TaskUseResult::NotInStage
        }
      }
      QueryHookStage::ResolveTask { task, .. } => {
        if *token != u32::MAX {
          TaskUseResult::Result(task.expect_result_by_id(*token))
        } else {
          TaskUseResult::NotInStage
        }
      }
      _ => TaskUseResult::NotInStage,
    }
  }

  /// warning, the delta/change must not shared
  fn use_shared_compute<Provider: SharedResultProvider<Self>>(
    &mut self,
    provider: Provider,
  ) -> UseResult<Provider::Result> {
    let key = provider.compute_share_key();
    let consumer_id = self.use_shared_consumer(key);
    self.use_shared_compute_internal(&|cx| provider.use_logic(cx), key, consumer_id)
  }

  fn use_shared_dual_query_internal<Provider: SharedResultProvider<Self, Result: DualQueryLike>>(
    &mut self,
    provider: Provider,
  ) -> (
    UseResult<MaterializedDeltaDualQuery<Provider::Result>>,
    ShareKey,
    u32,
  ) {
    let key = provider.compute_share_key();
    let consumer_id = self.use_shared_consumer(key);
    let r = self.use_shared_compute_internal(
      &|cx| {
        provider
          .use_logic(cx)
          .map_only_spawn_stage_in_thread_dual_query(cx, |r| r.materialize_delta())
      },
      key,
      consumer_id,
    );
    (r, key, consumer_id)
  }

  // todo, improve: the view only consumer not need reconcile, but still rely on
  // use_shared_dual_query to fanout changes
  fn use_shared_dual_query_view<Provider, K: CKey, V: CValue>(
    &mut self,
    provider: Provider,
  ) -> UseResult<BoxedDynQuery<K, V>>
  where
    Provider: SharedResultProvider<Self, Result: DualQueryLike<Key = K, Value = V>>,
  {
    self.use_shared_dual_query(provider).map(|r| r.view())
  }

  fn use_shared_dual_query<Provider, K: CKey, V: CValue>(
    &mut self,
    provider: Provider,
  ) -> UseResult<BoxedDynDualQuery<K, V>>
  where
    Provider: SharedResultProvider<Self, Result: DualQueryLike<Key = K, Value = V>>,
  {
    let (result, key, consumer_id) = self.use_shared_dual_query_internal(provider);

    let reconciler = self
      .shared_hook_ctx()
      .reconciler
      .entry(key)
      .or_insert_with(|| Arc::new(SharedQueryChangeReconciler::<K, V>::default()))
      .clone();

    result.map_only_spawn_stage_in_thread_dual_query(self, move |r| {
      let (view, delta) = r.view_delta();
      if let Some(new_delta) = reconciler.reconcile(consumer_id, Box::new(delta.into_boxed())) {
        DualQuery {
          view: view.into_boxed(),
          delta: *new_delta
            .downcast::<BoxedDynQuery<K, ValueChange<V>>>()
            .unwrap(),
        }
      } else {
        // expand the full view as delta
        let new_delta = view
          .iter_key_value()
          .map(|(k, v)| (k, ValueChange::Delta(v, None)))
          .collect::<FastHashMap<_, _>>();
        let new_delta = Arc::new(new_delta);

        DualQuery {
          view: view.into_boxed(),
          delta: new_delta.into_boxed(),
        }
      }
    })
  }

  fn use_shared_compute_internal<T>(
    &mut self,
    logic: &dyn Fn(&mut Self) -> UseResult<T>,
    key: ShareKey,
    consumer_id: u32,
  ) -> UseResult<T>
  where
    T: Clone + Send + Sync + 'static,
  {
    {
      let shared = self.shared_hook_ctx().shared.entry(key).or_default();
      let mut shared = shared.write();
      shared.consumer.entry(consumer_id).or_insert_with(|| true);
    }

    let r = if let Some(&task_id) = self.shared_hook_ctx().task_id_mapping.get(&key) {
      match self.stage() {
        QueryHookStage::SpawnTask { pool, .. } => {
          UseResult::SpawnStageFuture(pool.share_task_by_id(task_id))
        }
        QueryHookStage::ResolveTask { task, .. } => {
          UseResult::ResolveStageReady(task.expect_result_by_id(task_id))
        }
        _ => UseResult::NotInStage,
      }
    } else {
      self.enter_shared_ctx(key, |cx| {
        let result = logic(cx);
        let result = result.use_global_shared(cx);
        let (cx, self_id) = cx.use_plain_state(|| u32::MAX);
        if let TaskUseResult::SpawnId(task_id) = result {
          *self_id = task_id;
          cx.shared_hook_ctx().task_id_mapping.insert(key, task_id);
        } else {
          cx.shared_hook_ctx().task_id_mapping.insert(key, *self_id);
        }

        match &cx.stage() {
          QueryHookStage::SpawnTask { pool, .. } => {
            UseResult::SpawnStageFuture(pool.share_task_by_id(*self_id))
          }
          QueryHookStage::ResolveTask { task, .. } => {
            UseResult::ResolveStageReady(task.expect_result_by_id(*self_id))
          }
          _ => UseResult::NotInStage,
        }
      })
    };

    if self.is_spawning_stage() {
      let changed = {
        let mut shared = self.shared_hook_ctx().shared.get(&key).unwrap().write();
        shared.consumer.insert(consumer_id, false).unwrap()
      };

      if let QueryHookStage::SpawnTask {
        change_collector, ..
      } = self.stage()
        && changed
      {
        change_collector.notify_change();
      }
    }

    r
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32;

  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx;

  fn enter_shared_ctx<R>(&mut self, key: ShareKey, f: impl FnOnce(&mut Self) -> R) -> R {
    let shared = self.shared_hook_ctx().shared.get(&key).unwrap().clone();

    let mut shared = shared.write();

    let mut old = ChangeCollector::default();
    if let QueryHookStage::SpawnTask {
      change_collector, ..
    } = self.stage()
    {
      std::mem::swap(&mut old, change_collector);
    }

    let memory = &mut shared.memory;

    let r = unsafe {
      core::ptr::swap(self.memory_mut(), memory);
      let r = f(self);

      self.memory_mut().created = true;
      self.memory_mut().current_cursor = 0;
      self.memory_mut().flush_assume_only_plain_states();

      core::ptr::swap(self.memory_mut(), memory);
      r
    };

    if let QueryHookStage::SpawnTask {
      change_collector, ..
    } = self.stage()
    {
      let changed = change_collector.changed;
      std::mem::swap(&mut old, change_collector);
      if changed {
        change_collector.notify_change();
      }

      if changed {
        shared.consumer.values_mut().for_each(|v| *v = true);
      }
    }

    r
  }

  fn use_rev_ref<V: CKey, C: Query<Value = ValueChange<V>> + 'static>(
    &mut self,
    changes: UseResult<C>,
  ) -> UseResult<RevRefContainerRead<V, C::Key>> {
    let (_, mapping) = self.use_plain_state_default_cloned::<RevRefContainer<V, C::Key>>();

    changes.map_only_spawn_stage_in_thread(
      self,
      |changes| changes.is_empty(),
      move |changes| {
        bookkeeping_hash_relation(&mut mapping.write(), changes);
        mapping.make_read_holder()
      },
    )
  }
}

pub type RevRefContainer<K, V> = Arc<RwLock<FastHashMap<K, FastHashSet<V>>>>;
pub type RevRefContainerRead<K, V> = LockReadGuardHolder<FastHashMap<K, FastHashSet<V>>>;

#[derive(Default)]
pub struct SharedHooksCtx {
  shared: FastHashMap<ShareKey, Arc<RwLock<SharedHookObject>>>,
  task_id_mapping: FastHashMap<ShareKey, u32>,
  pub reconciler: FastHashMap<ShareKey, Arc<dyn ChangeReconciler>>,
  next_consumer: u32,
}

impl SharedHooksCtx {
  pub fn reset_visiting(&mut self) {
    self.task_id_mapping.clear();
    for r in self.reconciler.values() {
      r.reset();
    }
  }

  pub fn next_consumer_id(&mut self) -> u32 {
    let id = self.next_consumer;
    self.next_consumer += 1;
    id
  }

  pub fn drop_consumer(
    &mut self,
    token: SharedConsumerToken,
  ) -> Option<Arc<RwLock<SharedHookObject>>> {
    let SharedConsumerToken(id, key) = token;

    // this check is necessary because not all key need reconcile change
    if let Some(reconciler) = self.reconciler.get_mut(&key)
      && reconciler.remove_consumer(id)
    {
      self.reconciler.remove(&key);
    }

    let mut target = self.shared.get_mut(&key).unwrap().write();
    assert!(target.consumer.remove(&id).is_some());
    if target.consumer.is_empty() {
      drop(target);
      self.shared.remove(&key).unwrap().into()
    } else {
      None
    }
  }
}

#[derive(Clone, Copy)]
pub struct SharedConsumerToken(pub u32, pub ShareKey);

#[derive(Default)]
pub struct SharedHookObject {
  pub memory: FunctionMemory,
  /// map id to changed state
  pub consumer: FastHashMap<u32, bool>,
}

pub trait ChangeReconciler: Send + Sync {
  /// return None if the change should use full view expand
  fn reconcile(&self, id: u32, change: Box<dyn Any>) -> Option<Box<dyn Any>>;
  fn reset(&self);
  fn remove_consumer(&self, id: u32) -> bool;
}

pub struct SharedQueryChangeReconciler<K, V> {
  internal: Arc<RwLock<SharedQueryChangeReconcilerInternal<K, V>>>,
}

pub struct SharedQueryChangeReconcilerInternal<K, V> {
  consumers: FastHashMap<u32, Vec<BoxedDynQuery<K, ValueChange<V>>>>,
  has_broadcasted: bool,
}

impl<K: CKey, V: CValue> ChangeReconciler for SharedQueryChangeReconciler<K, V> {
  fn reconcile(&self, id: u32, change: Box<dyn Any>) -> Option<Box<dyn Any>> {
    // this lock introduce a blocking scope, but it's small and guaranteed to have forward progress
    let mut internal = self.internal.write();
    //  the first consumer get the result broadcast the result to others
    if !internal.has_broadcasted {
      let change = change
        .downcast::<BoxedDynQuery<K, ValueChange<V>>>()
        .unwrap();
      internal.has_broadcasted = true;

      if change.iter_key_value().next().is_some() {
        for (_, v) in internal.consumers.iter_mut() {
          v.push(change.clone());
        }
      }
    }

    if !internal.consumers.contains_key(&id) {
      internal.consumers.insert(id, Vec::default());
      return None;
    }
    let buffered_changes = std::mem::take(internal.consumers.get_mut(&id).unwrap());
    drop(internal);

    let r = Box::new(finalize_buffered_changes(buffered_changes)) as Box<dyn Any>;
    r.into()
  }

  fn reset(&self) {
    self.internal.write().has_broadcasted = false;
  }

  fn remove_consumer(&self, id: u32) -> bool {
    let mut internal = self.internal.write();
    // we should not assert remove, because not all consumer need reconcile change
    internal.consumers.remove(&id);
    internal.consumers.is_empty()
  }
}

impl<K, V> Default for SharedQueryChangeReconciler<K, V> {
  fn default() -> Self {
    Self {
      internal: Default::default(),
    }
  }
}
impl<K, V> Default for SharedQueryChangeReconcilerInternal<K, V> {
  fn default() -> Self {
    Self {
      consumers: Default::default(),
      has_broadcasted: false,
    }
  }
}

pub fn finalize_buffered_changes<K: CKey, V: CValue>(
  mut changes: Vec<BoxedDynQuery<K, ValueChange<V>>>,
) -> BoxedDynQuery<K, ValueChange<V>> {
  if changes.is_empty() {
    return Box::new(EmptyQuery::default());
  }

  if changes.len() == 1 {
    return changes.pop().unwrap();
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(
      &mut target,
      c.iter_key_value().map(|(k, v)| (k.clone(), v.clone())),
    );
  }

  if target.is_empty() {
    Box::new(EmptyQuery::default())
  } else {
    Box::new(Arc::new(target))
  }
}

fn merge_into_hashmap<K: CKey, V: CValue>(
  map: &mut FastHashMap<K, ValueChange<V>>,
  iter: impl Iterator<Item = (K, ValueChange<V>)>,
) {
  iter.for_each(|(k, v)| {
    if let Some(current) = map.get_mut(&k) {
      if !current.merge(&v) {
        map.remove(&k);
      }
    } else {
      map.insert(k, v.clone());
    }
  })
}
