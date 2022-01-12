use std::{cell::RefCell, rc::Rc};

use crate::ResourceWrapped;

pub type MeshInner<T> = ResourceWrapped<T>;

pub struct MeshCell<T> {
  pub inner: Rc<RefCell<MeshInner<T>>>,
}

impl<T> MeshCell<T> {
  pub fn new(mesh: T) -> Self {
    let mesh = MeshInner::new(mesh);
    Self {
      inner: Rc::new(RefCell::new(mesh)),
    }
  }
}

impl<T> Clone for MeshCell<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
