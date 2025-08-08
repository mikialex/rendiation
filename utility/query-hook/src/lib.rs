use std::any::Any;
use std::future::Future;
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
  Update { spawner: &'a TaskSpawner },
  CreateRender { task: &'a mut TaskPoolResultCx },
}

pub trait QueryHookCxLike: HooksCxLike {
  fn stage(&mut self) -> QueryHookStage;
  fn pool(&mut self) -> &mut AsyncTaskPool;

  fn use_task_result<R, F>(&mut self, create_task: impl Fn(&TaskSpawner) -> F) -> Option<R>
  where
    R: 'static,
    F: Future<Output = R> + Send + 'static,
  {
    let task = self.spawn_task_when_update(create_task);
    let (cx, token) = self.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::Update { .. } => {
        *token = cx.pool().install_task(task.unwrap());
        None
      }
      QueryHookStage::CreateRender { task, .. } => {
        let result = task
          .token_based_result
          .remove(token)
          .unwrap()
          .downcast()
          .unwrap();
        Some(*result)
      }
    }
  }

  fn spawn_task_when_update<R, F: Future<Output = R>>(
    &mut self,
    create_task: impl Fn(&TaskSpawner) -> F,
  ) -> Option<F> {
    match self.stage() {
      QueryHookStage::Update { spawner } => {
        let task = create_task(spawner);
        Some(task)
      }
      _ => None,
    }
  }
}
