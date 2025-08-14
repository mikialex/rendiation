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
}

pub enum TaskUseResult<T> {
  SpawnId(u32),
  Result(T),
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

#[derive(Default)]
pub struct SharedHookResult {
  pub task_id_mapping: FastHashMap<ShareKey, u32>,
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

pub trait QueryHookCxLike: HooksCxLike {
  fn is_spawning_stage(&self) -> bool;
  fn stage(&mut self) -> QueryHookStage;

  fn shared_ctx(&mut self) -> &mut SharedHookResult;

  fn when_spawning_stage(&self, f: impl FnOnce()) {
    if self.is_spawning_stage() {
      f();
    }
  }

  #[track_caller]
  fn use_result<T: Send + Sync + 'static + Clone>(&mut self, re: UseResult<T>) -> UseResult<T> {
    let (cx, spawned) = self.use_plain_state_default();
    if cx.is_spawning_stage() || *spawned {
      let fut = match re {
        UseResult::SpawnStageFuture(future) => {
          *spawned = true;
          Some(future)
        }
        UseResult::SpawnStageReady(re) => return UseResult::SpawnStageReady(re),
        _ => {
          if cx.is_spawning_stage() {
            panic!("must contain work in spawning stage")
          } else {
            None
          }
        }
      };

      cx.scope(|cx| cx.use_future(fut).into())
    } else {
      re
    }
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

  fn use_future<R: 'static + Send + Sync + Clone>(
    &mut self,
    f: Option<impl Future<Output = R> + Send + Sync + 'static>,
  ) -> TaskUseResult<R> {
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { pool, .. } => {
        *token = pool.install_task(f.unwrap());
        TaskUseResult::SpawnId(*token)
      }
      QueryHookStage::ResolveTask { task, .. } => {
        TaskUseResult::Result(task.expect_result_by_id(*token))
      }
    }
  }

  /// warning, the delta/change must not shared
  #[track_caller]
  fn use_shared_compute<Provider: SharedResultProvider<Self>>(
    &mut self,
    provider: Provider,
  ) -> UseResult<Provider::Result> {
    let key = provider.compute_share_key();
    self
      .use_shared_compute_internal(move |cx| provider.use_logic(cx), key)
      .0
  }

  #[track_caller]
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
    let (result, is_shared) =
      self.use_shared_compute_internal(move |cx| provider.use_logic(cx), key);

    result.map(move |r| {
      if is_shared {
        r.replace_delta_by_full_view().into_boxed()
      } else {
        r.into_boxed()
      }
    })
  }

  #[track_caller]
  fn use_shared_compute_internal<
    T: Clone + Send + Sync + 'static,
    F: Fn(&mut Self) -> TaskUseResult<T> + 'static,
  >(
    &mut self,
    logic: F,
    key: ShareKey,
  ) -> (UseResult<T>, bool) {
    if let Some(&task_id) = self.shared_ctx().task_id_mapping.get(&key) {
      let r = match &self.stage() {
        QueryHookStage::SpawnTask { pool, .. } => {
          UseResult::SpawnStageFuture(pool.share_task_by_id(task_id))
        }
        QueryHookStage::ResolveTask { task, .. } => {
          UseResult::ResolveStageReady(task.expect_result_by_id(task_id))
        }
      };
      (r, true)
    } else {
      self.scope(|cx| {
        let result = logic(cx);
        let (cx, self_id) = cx.use_plain_state(|| u32::MAX);
        if let TaskUseResult::SpawnId(task_id) = result {
          *self_id = task_id;
          cx.shared_ctx().task_id_mapping.insert(key, task_id);
        } else {
          cx.shared_ctx().task_id_mapping.insert(key, *self_id);
        }

        let r = match &cx.stage() {
          QueryHookStage::SpawnTask { pool, .. } => {
            UseResult::SpawnStageFuture(pool.share_task_by_id(*self_id))
          }
          QueryHookStage::ResolveTask { task, .. } => {
            UseResult::ResolveStageReady(task.expect_result_by_id(*self_id))
          }
        };
        (r, false)
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
