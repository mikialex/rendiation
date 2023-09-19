use std::sync::atomic::AtomicBool;

use crate::*;

#[derive(Default)]
pub struct NotifyScope {
  inner: Arc<NotifyScopeInner>,
}

struct NotifyScopeInner {
  waked: AtomicBool,
  upstream: Mutex<Option<Waker>>,
}

impl Default for NotifyScopeInner {
  fn default() -> Self {
    Self {
      waked: AtomicBool::new(true),
      upstream: Mutex::new(None),
    }
  }
}

impl NotifyScope {
  pub fn update_once(
    &self,
    upstream: Option<&mut Context>,
    update: impl FnOnce(&mut Context),
  ) -> bool {
    self.poll_changed_update(upstream.map(|cx| cx.waker()).cloned(), update)
  }

  pub fn update_total(&self, upstream: Option<&mut Context>, mut update: impl FnMut(&mut Context)) {
    let upstream = upstream.map(|cx| cx.waker()).cloned();
    loop {
      if !self.poll_changed_update(upstream.clone(), &mut update) {
        return;
      }
    }
  }

  pub fn notify_by(&self, upstream: Option<&mut Context>, logic: impl FnOnce(&mut Context)) {
    let upstream = upstream.map(|cx| cx.waker()).cloned();
    *self.inner.upstream.lock().unwrap() = upstream;

    let waker = futures::task::waker_ref(&self.inner);
    let mut cx = Context::from_waker(&waker);
    logic(&mut cx)
  }

  pub fn poll_changed_update(
    &self,
    upstream: Option<Waker>,
    update: impl FnOnce(&mut Context),
  ) -> bool {
    *self.inner.upstream.lock().unwrap() = upstream;

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

impl futures::task::ArcWake for NotifyScopeInner {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self
      .waked
      .fetch_or(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(upstream) = arc_self.upstream.lock().unwrap().take() {
      upstream.wake()
    }
  }
}
