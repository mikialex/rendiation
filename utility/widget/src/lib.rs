mod state;
use std::marker::PhantomData;

pub use state::*;

mod group;
use fast_hash_collection::*;
pub use group::*;

/// state lives in self(internal state) or cx(external state passed in)
/// view lives in self(self present) or cx(outside view provider passed in)
pub trait Widget {
  /// foreach frame, view react to event and change state, event info is input from cx
  fn update_state(&mut self, cx: &mut StateCx);
  /// foreach frame, after update_state, state sync change to view and present to user
  fn update_view(&mut self, cx: &mut StateCx);
  /// should be called before self drop, do resource cleanup within the same cx in update cycle
  fn clean_up(&mut self, cx: &mut StateCx);
}

impl Widget for () {
  fn update_state(&mut self, _: &mut StateCx) {}
  fn update_view(&mut self, _: &mut StateCx) {}
  fn clean_up(&mut self, _: &mut StateCx) {}
}

impl Widget for Box<dyn Widget> {
  fn update_state(&mut self, cx: &mut StateCx) {
    (**self).update_state(cx)
  }

  fn update_view(&mut self, cx: &mut StateCx) {
    (**self).update_view(cx)
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
    (**self).clean_up(cx)
  }
}

pub trait WidgetExt: Widget + Sized {
  fn with_view_update(self, f: impl FnMut(&mut Self, &mut StateCx)) -> impl Widget {
    ViewUpdate { inner: self, f }
  }
  fn with_state_update(self, f: impl FnMut(&mut StateCx)) -> impl Widget {
    StateUpdate {
      inner: self,
      f,
      post_update: false,
    }
  }
  fn with_state_post_update(self, f: impl FnMut(&mut StateCx)) -> impl Widget {
    StateUpdate {
      inner: self,
      f,
      post_update: true,
    }
  }
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl Widget {
    StateCtxInject { view: self, state }
  }
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl Widget {
    StateCtxPick {
      view: self,
      pick: len,
      phantom: PhantomData,
    }
  }
}

impl<T: Widget> WidgetExt for T {}

pub struct ViewUpdate<T, F> {
  inner: T,
  f: F,
}

impl<T: Widget, F: FnMut(&mut T, &mut StateCx)> Widget for ViewUpdate<T, F> {
  fn update_view(&mut self, cx: &mut StateCx) {
    (self.f)(&mut self.inner, cx);
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    self.inner.update_state(cx)
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.inner.clean_up(cx)
  }
}

pub struct StateUpdate<T, F> {
  inner: T,
  f: F,
  post_update: bool,
}

impl<T: Widget, F: FnMut(&mut StateCx)> Widget for StateUpdate<T, F> {
  fn update_view(&mut self, cx: &mut StateCx) {
    self.inner.update_view(cx)
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    if self.post_update {
      self.inner.update_state(cx);
      (self.f)(cx);
    } else {
      (self.f)(cx);
      self.inner.update_state(cx);
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.inner.clean_up(cx)
  }
}

pub struct StateCxCreateOnce<T, F> {
  inner: Option<T>,
  f: F,
}

impl<T, F: Fn(&mut StateCx) -> T> StateCxCreateOnce<T, F> {
  pub fn new(f: F) -> Self {
    Self { inner: None, f }
  }
}

impl<T: Widget, F: Fn(&mut StateCx) -> T> Widget for StateCxCreateOnce<T, F> {
  fn update_state(&mut self, cx: &mut StateCx) {
    let inner = self.inner.get_or_insert_with(|| (self.f)(cx));
    inner.update_state(cx)
  }
  fn update_view(&mut self, cx: &mut StateCx) {
    let inner = self.inner.get_or_insert_with(|| (self.f)(cx));
    inner.update_view(cx)
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    if let Some(inner) = &mut self.inner {
      inner.clean_up(cx)
    }
  }
}
