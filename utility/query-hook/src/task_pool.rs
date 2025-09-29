use dyn_clone::DynClone;
use futures::future::Shared;

use crate::*;

#[derive(Clone)]
pub struct TaskSpawner {
  num_threads: Option<usize>,
  #[cfg(not(target_family = "wasm"))]
  pool: Arc<rayon::ThreadPool>,
}

impl TaskSpawner {
  #[allow(unused_variables)]
  pub fn new(name: &'static str, num_threads: Option<usize>) -> Self {
    #[cfg(not(target_family = "wasm"))]
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

    Self {
      num_threads,
      #[cfg(not(target_family = "wasm"))]
      pool: Arc::new(pool),
    }
  }

  pub fn num_threads_config(&self) -> Option<usize> {
    self.num_threads
  }

  pub fn spawn_task<R: Send + Sync + 'static>(
    &self,
    f: impl FnOnce() -> R + Send + 'static,
  ) -> impl Future<Output = R> + Send + Sync + 'static {
    #[cfg(not(target_family = "wasm"))]
    {
      let (sender, receiver) = futures::channel::oneshot::channel();
      self.pool.spawn(move || {
        sender.send(f()).ok();
      });
      receiver.map(|v| v.expect("task unexpect cancelled"))
    }

    #[cfg(target_family = "wasm")]
    {
      std::future::ready(f())
    }
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
  token_based_result: FastHashMap<u32, Box<dyn AnyClone>>,
}

impl TaskPoolResultCx {
  pub fn expect_result_by_id<T: Clone + Any>(&self, id: u32) -> T {
    self
      .token_based_result
      .get(&id)
      .map(|v| v.deref().as_any().downcast_ref::<T>().unwrap().clone()) // todo, bad
      .unwrap()
  }
}

impl AsyncTaskPool {
  pub fn share_task_by_id<T: Clone + Any>(
    &self,
    id: u32,
  ) -> Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>> {
    let f = self
      .registry
      .get(&id)
      .unwrap()
      .clone()
      .map(|v| v.deref().as_any().downcast_ref::<T>().unwrap().clone()); // todo bad
    Box::pin(f)
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

  pub fn all_async_task_done(self) -> impl Future<Output = TaskPoolResultCx> {
    self
      .registry
      .into_iter()
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
