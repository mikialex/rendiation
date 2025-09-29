use std::{any::TypeId, collections::VecDeque};

use fast_hash_collection::FastHashMap;
use futures::{executor::LocalPool, task::LocalSpawnExt, Future};

use crate::*;

pub struct Terminal {
  console: Console,
  pub command_registry: FastHashMap<String, TerminalCommandCb>,
  pub executor: LocalPool,
  /// some task may only run on main thread, for example acquire db write lock
  pub main_thread_tasks: futures::channel::mpsc::UnboundedReceiver<Box<dyn FnOnce()>>,
  pub ctx: TerminalCtx,
  pub buffered_requests: VecDeque<String>,
}

#[derive(Clone)]
pub struct TerminalCtx {
  channel: futures::channel::mpsc::UnboundedSender<Box<dyn FnOnce()>>,
  pub(crate) store: TerminalTaskStore,
  pub worker: TaskSpawner,
}

pub trait TerminalTask: 'static {
  type Result: 'static;
}

#[derive(Default, Clone)]
pub struct TerminalTaskStore {
  store: Arc<RwLock<MessageStoreNoSendSync>>,
}

#[derive(Default)]
pub struct MessageStoreNoSendSync {
  messages: FastHashMap<TypeId, Box<dyn Any>>,
}

impl MessageStoreNoSendSync {
  pub fn put(&mut self, msg: impl Any) {
    self.messages.insert(msg.type_id(), Box::new(msg));
  }
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .messages
      .get(&TypeId::of::<T>())
      .as_ref()
      .map(|v| v.downcast_ref::<T>().unwrap())
  }
  pub fn take<T: Any>(&mut self) -> Option<T> {
    self
      .messages
      .remove(&TypeId::of::<T>())
      .map(|v| *v.downcast::<T>().unwrap())
  }
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

  pub fn spawn_main_thread<R: 'static>(
    &self,
    task: impl FnOnce() -> R + 'static,
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

impl Terminal {
  pub fn new(worker: TaskSpawner) -> Self {
    let (s, r) = futures::channel::mpsc::unbounded();
    let ctx = TerminalCtx {
      channel: s,
      store: Default::default(),
      worker,
    };

    Self {
      console: Console::new(),
      command_registry: Default::default(),
      executor: futures::executor::LocalPool::new(),
      main_thread_tasks: r,
      buffered_requests: Default::default(),
      ctx,
    }
  }
}

type TerminalCommandCb =
  Box<dyn Fn(&mut TerminalInitExecuteCx, &Vec<String>) -> Pin<Box<dyn Future<Output = ()>>>>;

pub struct TerminalInitExecuteCx<'a> {
  pub scene: &'a Viewer3dContent,
  pub renderer: &'a mut Viewer3dRenderingCtx,
  pub dyn_cx: &'a mut DynCx,
}

impl Terminal {
  pub fn egui(&mut self, ui: &mut egui::Ui) {
    if let Some(command) = self.console.ui(ui) {
      self.buffered_requests.push_back(command)
    }
  }

  pub fn tick_execute(&mut self, cx: &mut TerminalInitExecuteCx) {
    if let Some(command) = self.buffered_requests.pop_front() {
      self.execute_current(command, cx);
    }

    self.executor.run_until_stalled();

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
    FR: Future<Output = ()> + 'static,
    F: Fn(&mut TerminalInitExecuteCx, &Vec<String>, &TerminalCtx) -> FR + 'static,
  {
    let cx = self.ctx.clone();
    self.command_registry.insert(
      name.as_ref().to_owned(),
      Box::new(move |c, p| Box::pin(f(c, p, &cx))),
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
        self.executor.spawner().spawn_local(task).unwrap();
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
