use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};
use wasm_bindgen::prelude::*;

use crate::{NyxtViewerHandle, NyxtViewerInnerTrait, NyxtViewerMutableHandle};

#[derive(Clone)]
pub struct NyxtViewerHandledObject<V: NyxtViewerInnerTrait, Handle: NyxtViewerHandle<V>> {
  pub handle: Handle,
  pub inner: Weak<RefCell<V>>,
}

impl<V: NyxtViewerInnerTrait, Handle: NyxtViewerHandle<V>> NyxtViewerHandledObject<V, Handle> {
  pub fn new(inner: &Rc<RefCell<V>>, handle: Handle) -> Self {
    Self {
      inner: Rc::downgrade(inner),
      handle,
    }
  }

  pub fn mutate_inner<T>(&self, mutator: impl FnOnce(&mut V) -> T) -> T {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    let mut inner = inner.borrow_mut();
    mutator(&mut inner)
  }

  pub fn clone_viewer(&self) -> Rc<RefCell<V>> {
    Weak::upgrade(&self.inner).unwrap_throw()
  }
}

impl<V: NyxtViewerInnerTrait, Handle: NyxtViewerMutableHandle<V>>
  NyxtViewerHandledObject<V, Handle>
{
  pub fn mutate_item<T>(&self, mutator: impl FnOnce(&mut Handle::Item) -> T) -> T {
    let inner = Weak::upgrade(&self.inner).unwrap_throw();
    let mut inner = inner.borrow_mut();
    let item = self.handle.get_mut(&mut inner);
    mutator(item)
  }
}
