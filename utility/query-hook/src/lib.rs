#![feature(let_chains)]

use std::any::Any;
use std::any::TypeId;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;

use fast_hash_collection::*;
use futures::stream::*;
use futures::FutureExt;
use parking_lot::RwLock;
use query::*;

mod task_pool;
mod use_result;

pub use hook::*;
pub use task_pool::*;
pub use use_result::*;

pub enum QueryHookStage<'a> {
  SpawnTask {
    spawner: &'a TaskSpawner,
    pool: &'a mut AsyncTaskPool,
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

impl<T> TaskUseResult<T> {
  pub fn expect_result(self) -> T {
    match self {
      TaskUseResult::Result(v) => v,
      _ => panic!("expect result"),
    }
  }
  pub fn expect_id(self) -> u32 {
    match self {
      TaskUseResult::SpawnId(v) => v,
      _ => panic!("expect id"),
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
  fn use_logic(&self, cx: &mut Cx) -> TaskUseResult<Self::Result>;
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
  if let Some(changes) = change.if_ready() {
    if changes.has_change() {
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
}

pub trait QueryHookCxLike: HooksCxLike {
  fn is_spawning_stage(&self) -> bool;
  fn is_resolve_stage(&self) -> bool;
  fn stage(&mut self) -> QueryHookStage;

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

  fn use_task_result_by_fn<R, F>(&mut self, create_task: F) -> TaskUseResult<R>
  where
    R: Clone + Sync + Send + 'static,
    F: FnOnce() -> R + Send + 'static,
  {
    self.use_task_result(|spawner| spawner.spawn_task(create_task))
  }

  fn use_task_result<R, F>(
    &mut self,
    create_task: impl FnOnce(&TaskSpawner) -> F,
  ) -> TaskUseResult<R>
  where
    R: Clone + Send + Sync + 'static,
    F: Future<Output = R> + Send + 'static,
  {
    let task = self.spawn_task_when_update(create_task);
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { pool, .. } => {
        *token = pool.install_task(task.unwrap());
        TaskUseResult::SpawnId(*token)
      }
      QueryHookStage::ResolveTask { task, .. } => {
        TaskUseResult::Result(task.expect_result_by_id(*token))
      }
      _ => TaskUseResult::NotInStage,
    }
  }

  fn spawn_task_when_update<R, F: Future<Output = R>>(
    &mut self,
    create_task: impl FnOnce(&TaskSpawner) -> F,
  ) -> Option<F> {
    match self.stage() {
      QueryHookStage::SpawnTask { spawner, .. } => {
        let task = create_task(spawner);
        Some(task)
      }
      _ => None,
    }
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
    self.use_shared_compute_internal(move |cx| provider.use_logic(cx), key)
  }

  fn use_shared_dual_query_view<Provider: SharedResultProvider<Self, Result: DualQueryLike>>(
    &mut self,
    provider: Provider,
  ) -> UseResult<<Provider::Result as DualQueryLike>::View> {
    let key = provider.compute_share_key();
    let result = self.use_shared_compute_internal(move |cx| provider.use_logic(cx), key);

    result.map(|r| r.view()) // here we don't care to sync the change
  }

  fn use_shared_dual_query<Provider: SharedResultProvider<Self, Result: DualQueryLike>>(
    &mut self,
    provider: Provider,
  ) -> UseResult<
    BoxedDynDualQuery<
      <Provider::Result as DualQueryLike>::Key,
      <Provider::Result as DualQueryLike>::Value,
    >,
  > {
    let key = provider.compute_share_key();
    let (cx, has_synced_change) = self.use_plain_state_default_cloned::<Arc<RwLock<bool>>>();
    let result = cx.use_shared_compute_internal(move |cx| provider.use_logic(cx), key);

    result.map(move |r| {
      if !*has_synced_change.read() {
        *has_synced_change.write() = true;
        r.replace_delta_by_full_view().into_boxed()
      } else {
        r.into_boxed()
      }
    })
  }

  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx;

  fn enter_shared_ctx<R>(
    &mut self,
    key: ShareKey,
    consumer_id: u32,
    f: impl FnOnce(&mut Self) -> R,
  ) -> R {
    let shared = self
      .shared_hook_ctx()
      .shared
      .entry(key)
      .or_default()
      .clone();

    let mut shared = shared.write();
    shared.consumer.insert(consumer_id);
    let memory = &mut shared.memory;

    let r = unsafe {
      core::ptr::swap(self.memory_mut(), memory);
      let r = f(self);

      self.memory_mut().created = true;
      self.memory_mut().current_cursor = 0;
      self.flush();

      core::ptr::swap(self.memory_mut(), memory);
      r
    };

    r
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32;

  fn use_shared_compute_internal<
    T: Clone + Send + Sync + 'static,
    F: Fn(&mut Self) -> TaskUseResult<T> + 'static,
  >(
    &mut self,
    logic: F,
    key: ShareKey,
  ) -> UseResult<T> {
    let consumer_id = self.use_shared_consumer(key);

    if let Some(&task_id) = self.shared_hook_ctx().task_id_mapping.get(&key) {
      match &self.stage() {
        QueryHookStage::SpawnTask { pool, .. } => {
          UseResult::SpawnStageFuture(pool.share_task_by_id(task_id))
        }
        QueryHookStage::ResolveTask { task, .. } => {
          UseResult::ResolveStageReady(task.expect_result_by_id(task_id))
        }
        _ => UseResult::NotInStage,
      }
    } else {
      self.enter_shared_ctx(key, consumer_id, |cx| {
        let result = logic(cx);
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
    }
  }

  fn use_rev_ref<V: CKey, C: Query<Value = ValueChange<V>> + 'static>(
    &mut self,
    changes: UseResult<C>,
  ) -> TaskUseResult<RevRefContainerRead<V, C::Key>> {
    let (_, mapping) = self.use_plain_state_default_cloned::<RevRefContainer<V, C::Key>>();
    self.use_task_result_by_fn(move || {
      bookkeeping_hash_relation(&mut mapping.write(), changes.expect_spawn_stage_ready());
      mapping.make_read_holder()
    })
  }
}

pub type RevRefContainer<K, V> = Arc<RwLock<FastHashMap<K, FastHashSet<V>>>>;
pub type RevRefContainerRead<K, V> = LockReadGuardHolder<FastHashMap<K, FastHashSet<V>>>;

#[derive(Default)]
pub struct SharedHooksCtx {
  shared: FastHashMap<ShareKey, Arc<RwLock<SharedHookObject>>>,
  task_id_mapping: FastHashMap<ShareKey, u32>,
  next_consumer: u32,
}

impl SharedHooksCtx {
  pub fn reset_visiting(&mut self) {
    self.task_id_mapping.clear();
  }

  pub fn next_consumer_id(&mut self) -> u32 {
    let id = self.next_consumer;
    self.next_consumer += 1;
    id
  }

  pub fn drop_consumer(&mut self, key: ShareKey, id: u32) -> Option<Arc<RwLock<SharedHookObject>>> {
    let mut target = self.shared.get_mut(&key).unwrap().write();
    target.consumer.remove(&id);
    if target.consumer.is_empty() {
      drop(target);
      self.shared.remove(&key).unwrap().into()
    } else {
      None
    }
  }
}

#[derive(Default)]
pub struct SharedHookObject {
  pub memory: FunctionMemory,
  pub consumer: FastHashSet<u32>,
  // consumer: FastHashMap<u32, (bool, Option<Box<dyn Any>>)>, // bool: if care the change
}
