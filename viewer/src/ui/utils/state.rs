use std::{
  cell::RefCell,
  ops::{Deref, DerefMut},
  rc::Rc,
};

pub struct StateCell<T> {
  state: Rc<RefCell<T>>,
}

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
}

impl<T> Clone for StateCell<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
    }
  }
}
