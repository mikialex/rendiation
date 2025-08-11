use std::any::Any;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;

use fast_hash_collection::*;
use futures::stream::*;
use futures::FutureExt;

mod task_pool;

use hook::HooksCxLike;
pub use task_pool::*;

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

pub trait QueryHookCxLike: HooksCxLike {
  fn is_spawning_stage(&self) -> bool;
  fn stage(&mut self) -> QueryHookStage;

  fn when_spawning_stage(&self, f: impl FnOnce()) {
    if self.is_spawning_stage() {
      f();
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
    f: impl Future<Output = R> + Send + Sync + 'static,
  ) -> TaskUseResult<R> {
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { pool, .. } => {
        *token = pool.install_task(f);
        TaskUseResult::SpawnId(*token)
      }
      QueryHookStage::ResolveTask { task, .. } => {
        TaskUseResult::Result(task.expect_result_by_id(*token))
      }
    }
  }
}
