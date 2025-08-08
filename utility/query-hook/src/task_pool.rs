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

#[derive(Default)]
pub struct AsyncTaskPool {
  registry: FastHashMap<u32, Pin<Box<dyn Future<Output = Box<dyn Any>>>>>,
  next: u32,
}

#[derive(Default)]
pub struct TaskPoolResultCx {
  pub token_based_result: FastHashMap<u32, Box<dyn Any>>,
}

impl AsyncTaskPool {
  pub fn install_task<T: 'static>(
    &mut self,
    task: impl Future<Output = T> + Send + 'static,
  ) -> u32 {
    self.next += 1;

    let task = task.map(|v| Box::new(v) as Box<dyn Any>).boxed();

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
