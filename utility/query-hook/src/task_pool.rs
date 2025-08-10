use dyn_clone::DynClone;
use futures::future::Shared;

use crate::*;

pub struct TaskSpawner {
  pool: rayon::ThreadPool,
}

impl TaskSpawner {
  pub fn new(name: &'static str, num_threads: Option<usize>) -> Self {
    let pool = rayon::ThreadPoolBuilder::new()
      .thread_name(move |i| format!("{}-{}", name, i))
      .num_threads(
        num_threads.unwrap_or(
          std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        ),
      )
      .build()
      .unwrap();

    Self { pool }
  }

  pub fn spawn_task<R: Send + 'static>(
    &self,
    f: impl FnOnce() -> R + Send + 'static,
  ) -> impl Future<Output = R> + 'static {
    let (sender, receiver) = futures::channel::oneshot::channel();
    self.pool.spawn(move || {
      sender.send(f()).ok();
    });
    receiver.map(|v| v.expect("task unexpect cancelled"))
  }
}

pub trait AnyClone: Any + Sync + Send + DynClone + 'static {
  fn as_any(&self) -> &dyn Any;
}
impl<T> AnyClone for T
where
  T: Any + DynClone + Sync + Send + 'static,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
}
impl Clone for Box<dyn AnyClone> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

#[derive(Default)]
pub struct AsyncTaskPool {
  registry: FastHashMap<u32, Shared<Pin<Box<dyn Future<Output = Box<dyn AnyClone>> + Send>>>>,
  next: u32,
}

#[derive(Default)]
pub struct TaskPoolResultCx {
  pub token_based_result: FastHashMap<u32, Box<dyn AnyClone>>,
}

impl AsyncTaskPool {
  pub fn share_task_by_id<T: Clone + Any>(
    &self,
    id: u32,
  ) -> impl Future<Output = T> + Send + 'static {
    self
      .registry
      .get(&id)
      .unwrap()
      .clone()
      .map(|v| v.deref().as_any().downcast_ref::<T>().unwrap().clone()) // todo bad
  }

  pub fn install_task<T: 'static + Clone + Sync + Send>(
    &mut self,
    task: impl Future<Output = T> + Send + 'static,
  ) -> u32 {
    self.next += 1;

    let task = task
      .map(|v| Box::new(v) as Box<dyn AnyClone>)
      .boxed()
      .shared();

    self.registry.insert(self.next, task);
    self.next
  }

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
