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

impl futures::task::ArcWake for NotifyScopeImpl {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self
      .waked
      .fetch_or(true, std::sync::atomic::Ordering::SeqCst);
    arc_self.upstream.wake()
  }
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
  /// trigger the waker manually
  pub fn wake(&self) {
    futures::task::ArcWake::wake_by_ref(&self.inner);
  }

  pub fn run_if_previous_waked<R>(
    &self,
    cx: &mut Context,
    logic: impl FnOnce(&mut Context) -> R,
  ) -> Option<R> {
    self.inner.upstream.register(cx.waker());
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
      Some(logic(&mut cx))
    } else {
      None
    }
  }

  pub fn run_and_return_previous_waked<R>(
    &self,
    cx: &mut Context,
    logic: impl FnOnce(&mut Context) -> R,
  ) -> (bool, R) {
    self.inner.upstream.register(cx.waker());
    let waked_before = self
      .inner
      .waked
      .compare_exchange(
        true,
        false,
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
      )
      .is_ok();

    let waker = futures::task::waker_ref(&self.inner);
    let mut cx = Context::from_waker(&waker);
    let r = logic(&mut cx);
    (waked_before, r)
  }
}
