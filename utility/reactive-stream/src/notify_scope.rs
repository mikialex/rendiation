use std::sync::atomic::AtomicBool;

use crate::*;

#[derive(Default)]
pub struct NotifyScope {
  inner: Arc<NotifyScopeImpl>,
}

struct NotifyScopeImpl {
  waked: AtomicBool,
  upstream: AtomicWaker,
}

impl Default for NotifyScopeImpl {
  fn default() -> Self {
    Self {
      waked: AtomicBool::new(true),
      upstream: Default::default(),
    }
  }
}

impl NotifyScope {
  pub fn setup_waker(&self, upstream: Option<&mut Context>) {
    if let Some(cx) = upstream {
      self.inner.upstream.register(cx.waker());
    }
  }

  pub fn wake(&self) {
    futures::task::ArcWake::wake_by_ref(&self.inner);
  }
  pub fn update_once(&self, update: impl FnOnce(&mut Context)) -> bool {
    self.poll_changed_update(update)
  }

  pub fn update_total(&self, mut update: impl FnMut(&mut Context)) -> bool {
    let mut any_change = false;
    loop {
      if !self.poll_changed_update(&mut update) {
        return any_change;
      } else {
        any_change = true
      }
    }
  }

  pub fn notify_by(&self, logic: impl FnOnce(&mut Context)) {
    let waker = futures::task::waker_ref(&self.inner);
    let mut cx = Context::from_waker(&waker);
    logic(&mut cx)
  }

  fn poll_changed_update(&self, update: impl FnOnce(&mut Context)) -> bool {
    if self
      .inner
      .waked
      .compare_exchange(
        true,
        false,
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
      )
      .is_ok()
    {
      let waker = futures::task::waker_ref(&self.inner);
      let mut cx = Context::from_waker(&waker);
      update(&mut cx);
      true
    } else {
      false
    }
  }
}

impl futures::task::ArcWake for NotifyScopeImpl {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self
      .waked
      .fetch_or(true, std::sync::atomic::Ordering::SeqCst);
    arc_self.upstream.wake()
  }
}
