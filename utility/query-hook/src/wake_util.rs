use crate::*;

pub type ChangeNotifier = Arc<ChangeNotifierInternal>;

pub struct ChangeNotifierInternal {
  changed: AtomicBool,
  waker: futures::task::AtomicWaker,
}

impl ChangeNotifierInternal {
  pub fn update(&self, waker: &Waker) -> bool {
    let waked = self.changed.load(std::sync::atomic::Ordering::SeqCst);
    if waked {
      self
        .changed
        .store(false, std::sync::atomic::Ordering::SeqCst);
    }
    self.waker.register(waker);
    waked
  }

  pub fn do_wake(&self) {
    self
      .changed
      .store(true, std::sync::atomic::Ordering::SeqCst);
    self.waker.wake();
  }
}

impl Default for ChangeNotifierInternal {
  fn default() -> Self {
    Self {
      changed: AtomicBool::new(true),
      waker: Default::default(),
    }
  }
}

impl futures::task::ArcWake for ChangeNotifierInternal {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self.do_wake()
  }
}

#[derive(Default)]
pub struct BroadcastWaker {
  downstream: RwLock<FastHashMap<u32, Waker>>,
  has_notified_all: AtomicBool,
}

impl BroadcastWaker {
  pub fn setup(&self, id: u32, waker: Waker) {
    self.downstream.write().insert(id, waker);
    self
      .has_notified_all
      .store(false, std::sync::atomic::Ordering::SeqCst);
  }

  pub fn remove(&self, id: u32) -> bool {
    self.downstream.write().remove(&id).is_some()
  }
}

impl futures::task::ArcWake for BroadcastWaker {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    let should_wake = !arc_self
      .has_notified_all
      .load(std::sync::atomic::Ordering::SeqCst);
    if should_wake {
      arc_self
        .has_notified_all
        .store(true, std::sync::atomic::Ordering::SeqCst);
      arc_self
        .downstream
        .read()
        .iter()
        .for_each(|(_, w)| w.wake_by_ref());
    }
  }
}
