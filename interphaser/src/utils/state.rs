use std::{
  cell::RefCell,
  ops::{Deref, DerefMut},
  rc::Rc,
};

use crate::EventHandleCtx;

pub struct StateCell<T> {
  state: Rc<RefCell<T>>,
}

pub trait StateCreator: Default {
  fn use_state() -> StateCell<Self> {
    StateCell::new(Default::default())
  }
}
impl<T: Default> StateCreator for T {}

impl<T> StateCell<T> {
  pub fn new(state: T) -> Self {
    Self {
      state: Rc::new(RefCell::new(state)),
    }
  }

  pub fn visit<R, F: Fn(&T) -> R>(&self, f: F) -> R {
    f(self.state.borrow().deref())
  }

  pub fn mutate(&self, f: impl Fn(&mut T)) {
    f(self.state.borrow_mut().deref_mut())
  }

  pub fn mutator(&self, f: impl Fn(&mut T) + Copy) -> impl Fn() {
    let self_clone = self.clone();
    move || {
      self_clone.mutate(f);
    }
  }

  pub fn on_event<X, E>(
    &self,
    f: impl Fn(&mut T, &mut EventHandleCtx, &E) + Copy,
  ) -> impl Fn(&mut X, &mut EventHandleCtx, &E) {
    let self_clone = self.clone();
    move |_x: &mut X, ctx: &mut EventHandleCtx, event: &E| {
      f(self_clone.state.borrow_mut().deref_mut(), ctx, event)
    }
  }

  pub fn on_event_trigger<X, E>(
    &self,
    f: impl Fn(&mut T) + Copy,
  ) -> impl Fn(&mut X, &mut EventHandleCtx, &E) {
    self.on_event(move |s, _, _| f(s))
  }
}

impl<T> Clone for StateCell<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
    }
  }
}
