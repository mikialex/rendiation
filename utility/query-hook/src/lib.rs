#![feature(allocator_api)]

use std::any::Any;
use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::task::Context;
use std::task::Waker;

use fast_hash_collection::*;
use futures::FutureExt;
use futures::stream::*;
use parking_lot::RwLock;
pub use query::*;

mod frame_allocator;
mod task_pool;
mod use_result;
mod wake_util;

pub use frame_allocator::*;
pub use hook::*;
pub use task_pool::*;
pub use use_result::*;
pub use wake_util::*;

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
    immediate_results: &'a mut FastHashMap<u32, Arc<dyn Any + Send + Sync>>,
    change_collector: &'a mut ChangeCollector,
  },
  ResolveTask {
    task: &'a mut TaskPoolResultCx,
  },
  Other,
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
  fn waker(&mut self) -> &mut Waker;
  fn poll_ctx<R>(&mut self, f: impl FnOnce(&mut Context) -> R) -> R {
    let mut ctx = futures::task::Context::from_waker(self.waker());
    f(&mut ctx)
  }

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

  /// warning, the delta/change must not shared
  fn use_shared_compute<Provider: SharedResultProvider<Self>>(
    &mut self,
    provider: Provider,
  ) -> UseResult<Provider::Result> {
    let key = provider.compute_share_key();
    let consumer_id = self.use_shared_consumer(key);
    self.use_shared_compute_internal(&|cx| provider.use_logic(cx), key, consumer_id)
  }

  fn use_shared_dual_query_view<Provider, K: CKey, V: CValue>(
    &mut self,
    provider: Provider,
  ) -> UseResult<BoxedDynQuery<K, V>>
  where
    Provider: SharedResultProvider<Self, Result: DualQueryLike<Key = K, Value = V>>,
  {
    self
      .use_shared_dual_query_internal(provider, true)
      .map(|r| r.view())
  }

  fn use_shared_dual_query<Provider, K: CKey, V: CValue>(
    &mut self,
    provider: Provider,
  ) -> UseResult<BoxedDynDualQuery<K, V>>
  where
    Provider: SharedResultProvider<Self, Result: DualQueryLike<Key = K, Value = V>>,
  {
    self.use_shared_dual_query_internal(provider, false)
  }

  /// note, skip_change will still wake correctly
  fn use_shared_dual_query_internal<Provider, K: CKey, V: CValue>(
    &mut self,
    provider: Provider,
    skip_change: bool,
  ) -> UseResult<BoxedDynDualQuery<K, V>>
  where
    Provider: SharedResultProvider<Self, Result: DualQueryLike<Key = K, Value = V>>,
  {
    let key = provider.compute_share_key();
    let consumer_id = self.use_shared_consumer(key);
    let result = self.use_shared_compute_internal(
      &|cx| {
        provider
          .use_logic(cx)
          .map_spawn_stage_in_thread_dual_query(cx, |r| r.materialize_delta())
      },
      key,
      consumer_id,
    );

    let reconciler = self
      .shared_hook_ctx()
      .reconciler
      .entry(key)
      .or_insert_with(|| Arc::new(SharedQueryChangeReconciler::<K, V>::default()))
      .clone();

    result.map_spawn_stage_in_thread_dual_query(self, move |r| {
      let (view, delta) = r.view_delta();
      let delta = Box::new(delta.into_boxed());
      if let Some(new_delta) = reconciler.reconcile(consumer_id, delta, skip_change) {
        DualQuery {
          view: view.into_boxed(),
          delta: *new_delta
            .downcast::<BoxedDynQuery<K, ValueChange<V>>>()
            .unwrap(),
        }
      } else if skip_change {
        DualQuery {
          view: view.into_boxed(),
          delta: EmptyQuery::default().into_boxed(),
        }
      } else {
        // expand the full view as delta
        let new_delta = view
          .iter_key_value()
          .map(|(k, v)| (k, ValueChange::Delta(v, None)))
          .collect::<QueryMaterializedFastIter<K, ValueChange<V>>>();
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
    let logic = |cx: &mut Self| logic(cx).map(|v| Arc::new(v) as Arc<dyn Any + Send + Sync>);
    let r = self.use_shared_compute_internal_dyn(&logic, key, consumer_id);
    r.map(|v| v.downcast_ref::<T>().unwrap().clone())
  }

  #[inline(never)]
  fn use_shared_compute_internal_dyn(
    &mut self,
    logic: &dyn Fn(&mut Self) -> UseResult<Arc<dyn Any + Send + Sync>>,
    key: ShareKey,
    consumer_id: u32,
  ) -> UseResult<Arc<dyn Any + Send + Sync>> {
    let shared_waker = {
      let waker = self.waker().clone();
      let shared = self.shared_hook_ctx().shared.entry(key).or_default();
      let mut shared = shared.write();
      shared.consumer.entry(consumer_id).or_insert_with(|| true);
      shared.consumer_wakers.setup(consumer_id, waker);
      shared.consumer_wakers.clone()
    };

    if self.shared_hook_ctx().task_id_mapping.get(&key).is_none() {
      let waker_backup = self.waker().clone();
      *self.waker() = futures::task::waker(shared_waker);

      self.enter_shared_ctx(key, |cx| {
        let result = logic(cx);

        let (cx, persist_upstream_task_id) = cx.use_plain_state(|| u32::MAX);

        match result {
          UseResult::SpawnStageFuture(future) => {
            if let QueryHookStage::SpawnTask { pool, .. } = cx.stage() {
              let spawned_task_id = pool.install_task_dyn(future);
              cx.shared_hook_ctx()
                .task_id_mapping
                .insert(key, spawned_task_id);
              *persist_upstream_task_id = spawned_task_id;
            } else {
              unreachable!()
            };
          }
          UseResult::SpawnStageReady(result) => {
            // in this case, we have result in spawn stage directly in upstream
            // here we get an adhoc_per_cycle_unique_id task id.
            let some_id = u32::MAX / 2 - cx.shared_hook_ctx().task_id_mapping.len() as u32 - 1;
            cx.shared_hook_ctx().task_id_mapping.insert(key, some_id);
            *persist_upstream_task_id = some_id;
            if let QueryHookStage::SpawnTask {
              immediate_results, ..
            } = cx.stage()
            {
              immediate_results.insert(some_id, result);
            } else {
              unreachable!()
            };
          }
          UseResult::ResolveStageReady(result) => {
            // in this case, we have result in resolve stage directly in upstream
            // here we get an unique task id.
            let some_id = u32::MAX - cx.shared_hook_ctx().task_id_mapping.len() as u32 - 1;
            cx.shared_hook_ctx().task_id_mapping.insert(key, some_id);
            if let QueryHookStage::ResolveTask { task, .. } = cx.stage() {
              task.token_based_result.insert(some_id, result);
            } else {
              unreachable!()
            };
          }
          UseResult::NotInStage => {
            // we set it to u32 max to mark we have already check the upstream
            // if persist_upstream_task_id is not u32 max, it means we should use it as current task id
            // to expect_result_by_id for downstream in resolve stage
            cx.shared_hook_ctx()
              .task_id_mapping
              .insert(key, *persist_upstream_task_id);
            *persist_upstream_task_id = u32::MAX;
          }
        };
      });

      *self.waker() = waker_backup;
    }

    let task_id = *self.shared_hook_ctx().task_id_mapping.get(&key).unwrap();

    // if we enter this, the logic has already been executed in this stage before, so
    // we just share the task or clone the result.
    let r = match self.stage() {
      QueryHookStage::SpawnTask {
        pool,
        immediate_results,
        ..
      } => {
        if let Some(r) = immediate_results.get(&task_id) {
          UseResult::SpawnStageReady(r.clone())
        } else if let Some(f) = pool.try_share_task_by_id_dyn(task_id) {
          UseResult::SpawnStageFuture(f)
        } else {
          // this is possible if upstream not spawn or resolve anything in spawn stage
          UseResult::NotInStage
        }
      }
      QueryHookStage::ResolveTask { task, .. } => {
        if task_id == u32::MAX {
          UseResult::NotInStage
        } else {
          UseResult::ResolveStageReady(task.expect_result_by_id_any(task_id).clone())
        }
      }
      _ => UseResult::NotInStage,
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

    changes.map_spawn_stage_in_thread(
      self,
      |changes| changes.is_empty(),
      move |changes| {
        bookkeeping_hash_relation(&mut mapping.write(), changes);
        mapping.make_read_holder()
      },
    )
  }

  #[track_caller]
  fn skip_if_not_waked<R: Default>(&mut self, f: impl FnOnce(&mut Self) -> R) -> (bool, R) {
    let (cx, notifier) = self.use_plain_state(|| Arc::new(ChangeNotifierInternal::default()));
    let waked = notifier.update(cx.waker());
    let mut waker_backup = None;
    if waked {
      let waker = futures::task::waker(notifier.clone());
      waker_backup = cx.waker().clone().into();
      *cx.waker() = waker
    }

    let r = cx.skip_if_not(waked, f);

    if let Some(waker) = waker_backup {
      *cx.waker() = waker
    }

    // if spawn stage not skipped, we keep the resolve stage exist
    if waked && cx.is_spawning_stage() {
      notifier.do_wake();
    }

    (waked, r)
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
    assert!(target.consumer_wakers.remove(id));
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
  pub consumer_wakers: Arc<BroadcastWaker>,
}

pub trait ChangeReconciler: Send + Sync {
  /// return None if the change should use full view expand or skip_change = true
  fn reconcile(&self, id: u32, change: Box<dyn Any>, skip_change: bool) -> Option<Box<dyn Any>>;
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
  fn reconcile(&self, id: u32, change: Box<dyn Any>, skip_change: bool) -> Option<Box<dyn Any>> {
    // this lock introduce a blocking scope, but it's small and guaranteed to have forward progress
    let mut internal = self.internal.write();
    //  the first consumer get the result broadcast the result to others
    if !internal.has_broadcasted {
      let change = change
        .downcast::<BoxedDynQuery<K, ValueChange<V>>>()
        .unwrap();
      internal.has_broadcasted = true;

      if !change.is_empty() {
        for (_, v) in internal.consumers.iter_mut() {
          v.push(*change.clone());
        }
      }
    }

    if skip_change {
      // this also skips consumers.insert, which avoid memory leak
      return None;
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
    return Arc::new(EmptyQuery::default());
  }

  if changes.len() == 1 {
    return changes.pop().unwrap();
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(&mut target, c.iter_key_value());
  }

  if target.is_empty() {
    Arc::new(EmptyQuery::default())
  } else {
    Arc::new(target)
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
