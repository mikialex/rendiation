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

pub struct QueryHookCx<'a> {
  pub task_pool: &'a mut AsyncTaskPool,
}

pub enum QueryHookStage<'a> {
  SpawnTask { spawner: &'a TaskSpawner },
  ResolveTask { task: &'a mut TaskPoolResultCx },
}

pub trait QueryHookCxLike: HooksCxLike {
  fn is_spawning_stage(&self) -> bool;
  fn stage(&mut self) -> QueryHookStage;
  fn pool(&mut self) -> &mut AsyncTaskPool;

  fn when_spawning_stage(&self, f: impl FnOnce()) {
    if self.is_spawning_stage() {
      f();
    }
  }

  fn use_task_result_by_fn<R, F>(&mut self, create_task: F) -> Option<R>
  where
    R: Clone + Send + 'static,
    F: FnOnce() -> R + Send + 'static,
  {
    self.use_task_result(|spawner| spawner.spawn_task(create_task))
  }

  fn use_task_result<R, F>(&mut self, create_task: impl FnOnce(&TaskSpawner) -> F) -> Option<R>
  where
    R: Clone + 'static,
    F: Future<Output = R> + Send + 'static,
  {
    let task = self.spawn_task_when_update(create_task);
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { .. } => {
        *token = cx.pool().install_task(task.unwrap());
        None
      }
      QueryHookStage::ResolveTask { task, .. } => {
        let result = task
          .token_based_result
          .remove(token)
          .unwrap()
          .deref()
          .as_any()
          .downcast_ref::<R>()
          .unwrap()
          .clone(); // todo this is not good
        Some(result)
      }
    }
  }

  fn spawn_task_when_update<R, F: Future<Output = R>>(
    &mut self,
    create_task: impl FnOnce(&TaskSpawner) -> F,
  ) -> Option<F> {
    match self.stage() {
      QueryHookStage::SpawnTask { spawner } => {
        let task = create_task(spawner);
        Some(task)
      }
      _ => None,
    }
  }
}
