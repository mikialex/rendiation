use std::{cell::RefCell, rc::Weak};

use rendiation_ral::{ResourceManager, UniformHandle};

use crate::GFX;

pub struct UBONyxtWrap<T> {
  handle: UniformHandle<GFX, T>,
  resource: Weak<RefCell<ResourceManager<GFX>>>,
}

impl<T> UBONyxtWrap<T> {
  pub fn mutate<R>(&self, mutator: impl FnOnce(&mut T) -> R) -> R {
    todo!()
  }
}
