use std::{
  cell::RefCell,
  ops::{Deref, DerefMut},
  rc::Rc,
};

use crate::EventHandleCtx;

pub struct StateCell<T> {
  state: Rc<RefCell<T>>,
}

pub struct VersionedCell<T> {
  inner: T,
  version: u32,
}

impl<T> Deref for VersionedCell<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> DerefMut for VersionedCell<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.version += 1;
    &mut self.inner
  }
}

impl<T> VersionedValue for StateCell<VersionedCell<T>> {
  fn get_version(&self) -> u32 {
    self.state.borrow().version
  }
}

impl<T: 'static> StateCell<VersionedCell<T>> {
  pub fn boxed(&self) -> Box<dyn VersionedValue> {
    Box::new(self.clone())
  }
}

pub trait VersionedValue {
  fn get_version(&self) -> u32;
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

  pub fn mutate<E>(
    &self,
    f: impl Fn(&mut T, &mut EventHandleCtx, &E),
    ctx: &mut EventHandleCtx,
    event: &E,
  ) {
    f(self.state.borrow_mut().deref_mut(), ctx, event)
  }

  pub fn mutator<E>(
    &self,
    f: impl Fn(&mut T, &mut EventHandleCtx, &E) + Copy,
  ) -> impl Fn(&mut EventHandleCtx, &E) {
    let self_clone = self.clone();
    move |ctx: &mut EventHandleCtx, event: &E| {
      self_clone.mutate(f, ctx, event);
    }
  }

  pub fn mutation<X, E>(
    &self,
    f: impl Fn(&mut T, &mut EventHandleCtx, &E) + Copy,
  ) -> impl Fn(&mut X, &mut EventHandleCtx, &E) {
    let mutator = self.mutator(f);
    move |_x: &mut X, ctx: &mut EventHandleCtx, event: &E| mutator(ctx, event)
  }
}

impl<T> Clone for StateCell<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
    }
  }
}
