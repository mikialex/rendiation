use std::any::Any;

use futures::stream::*;
use futures::FutureExt;

use crate::*;

#[derive(Default)]
pub struct AsyncTaskPool {
  registry: FastHashMap<u32, Box<dyn Future<Output = Box<dyn Any>> + Unpin>>,
  next: u32,
}

#[derive(Default)]
pub struct TaskPoolResultCx {
  pub token_based_result: FastHashMap<u32, Box<dyn Any>>,
}

impl AsyncTaskPool {
  pub fn all_async_task_done(&mut self) -> impl Future<Output = TaskPoolResultCx> {
    self
      .registry
      .drain()
      .map(|(k, source)| source.map(move |r| (k, r)))
      .collect::<FuturesUnordered<_>>()
      .fold(
        TaskPoolResultCx::default(),
        |mut results, (k, result)| async move {
          results.token_based_result.insert(k, result);
          results
        },
      )
  }
}
