use fast_hash_collection::FastHashMap;
use futures::{executor::ThreadPool, Future};

use crate::*;

pub struct Terminal {
  console: Console,
  pub command_registry: FastHashMap<String, TerminalCommandCb>,
  pub executor: ThreadPool,
  /// some task may only run on main thread, for example acquire db write lock
  pub main_thread_tasks: futures::channel::mpsc::UnboundedReceiver<Box<dyn FnOnce() + Send + Sync>>,
  pub ctx: TerminalCtx,
}

#[derive(Clone)]
pub struct TerminalCtx {
  channel: futures::channel::mpsc::UnboundedSender<Box<dyn FnOnce() + Send + Sync>>,
  pub(crate) store: TerminalTaskStore,
}

pub trait TerminalTask: 'static + Send + Sync {
  type Result: 'static + Send + Sync;
}

#[derive(Default, Clone)]
pub struct TerminalTaskStore {
  store: Arc<RwLock<MessageStore>>,
}

pub struct TerminalTaskObject<T: TerminalTask> {
  phantom: std::marker::PhantomData<T>,
  sender: futures::channel::oneshot::Sender<T::Result>,
}

impl<T: TerminalTask> TerminalTaskObject<T> {
  pub fn resolve(self, result: T::Result) {
    self.sender.send(result).ok();
    //
  }
}

impl TerminalTaskStore {
  pub fn take<T: TerminalTask>(&mut self) -> Option<TerminalTaskObject<T>> {
    self.store.write().take::<TerminalTaskObject<T>>()
  }
}

impl TerminalCtx {
  pub fn spawn_event_task<R: TerminalTask>(&self) -> impl Future<Output = Option<R::Result>> {
    let (s, r) = futures::channel::oneshot::channel();
    self.store.store.write().put(TerminalTaskObject::<R> {
      phantom: std::marker::PhantomData,
      sender: s,
    });
    r.map(|v| v.ok())
  }

  pub fn spawn_main_thread<R: 'static + Send + Sync>(
    &self,
    task: impl FnOnce() -> R + Send + Sync + 'static,
  ) -> impl Future<Output = Option<R>> {
    let (s, r) = futures::channel::oneshot::channel();
    self
      .channel
      .unbounded_send(Box::new(|| {
        let result = task();
        s.send(result).ok();
      }))
      .ok();
    r.map(|v| v.ok())
  }
}

impl Default for Terminal {
  fn default() -> Self {
    let (s, r) = futures::channel::mpsc::unbounded();
    let ctx = TerminalCtx {
      channel: s,
      store: Default::default(),
    };

    Self {
      console: Console::new(),
      command_registry: Default::default(),
      executor: futures::executor::ThreadPool::builder()
        .name_prefix("viewer_terminal_task_thread")
        .pool_size(1)
        .create()
        .unwrap(),
      main_thread_tasks: r,
      ctx,
    }
  }
}

type TerminalCommandCb = Box<
  dyn Fn(&mut TerminalInitExecuteCx, &Vec<String>) -> Box<dyn Future<Output = ()> + Send + Unpin>,
>;

pub struct TerminalInitExecuteCx<'a> {
  pub scene: &'a Viewer3dSceneCtx,
  pub renderer: &'a mut Viewer3dRenderingCtx,
  pub dyn_cx: &'a mut DynCx,
}

impl Terminal {
  pub fn egui(&mut self, ui: &mut egui::Ui, cx: &mut TerminalInitExecuteCx) {
    let console_response = self.console.ui(ui);
    if let Some(command) = console_response {
      self.execute_current(command, cx);
    }

    noop_ctx!(ctx);
    while let Poll::Ready(Some(task)) = self.main_thread_tasks.poll_next_unpin(ctx) {
      task()
    }
  }

  pub fn unregister_command(&mut self, name: impl AsRef<str>) {
    self.command_registry.remove(name.as_ref());
  }

  pub fn register_command<F, FR>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    FR: Future<Output = ()> + Send + 'static,
    F: Fn(&mut TerminalInitExecuteCx, &Vec<String>, &TerminalCtx) -> FR + 'static,
  {
    let cx = self.ctx.clone();
    self.command_registry.insert(
      name.as_ref().to_owned(),
      Box::new(move |c, p| Box::new(Box::pin(f(c, p, &cx)))),
    );
    self
  }

  pub fn register_sync_command<F>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    F: Fn(&mut TerminalInitExecuteCx, &Vec<String>) + 'static + Send + Sync,
  {
    self.register_command(name, move |c, p, _| {
      f(c, p);
      async {}
    });
    self
  }

  pub fn execute_current(&mut self, command: String, ctx: &mut TerminalInitExecuteCx) {
    let parameters: Vec<String> = command
      .split_ascii_whitespace()
      .map(|s| s.to_owned())
      .collect();

    if let Some(command_name) = parameters.first() {
      if let Some(exe) = self.command_registry.get(command_name) {
        let task = exe(ctx, &parameters);
        self.executor.spawn_ok(task);
      } else {
        self
          .console
          .writeln(format!("unknown command {command_name}"));
      }
    }
  }
}

pub fn register_default_commands(terminal: &mut Terminal) {
  // this mainly to do test
  terminal.register_sync_command("clear-gpu-resource-cache", |ctx, _parameters| {
    let gpu = ctx.renderer.gpu();
    println!(
      "current gpu resource cache details: {:?}",
      gpu.create_cache_report()
    );
    gpu.clear_resource_cache();
  });
}
