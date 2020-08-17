use rendiation_ral::RALBackend;
use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
};

pub struct UBOManager<T: RALBackend> {
  data: HashMap<TypeId, Box<dyn UBOStorageTrait<dyn Any>>>,
  phantom: PhantomData<T>,
}

impl<T: RALBackend> UBOManager<T> {
  pub fn new() -> Self {
    Self {
      data: HashMap::new(),
      phantom: PhantomData,
    }
  }
}

trait UBOStorageTrait<T> {
  fn insert(&mut self, value: T) -> usize;
  fn delete(&mut self, handle: usize);
}

impl<T: RALBackend, U> UBOStorageTrait<U> for UBOStorage<T, U> {
  fn insert(&mut self, value: U) -> usize {
    let result = self.storage.len();
    self.storage.push(value);
    self.dirty = true;
    result
  }

  fn delete(&mut self, handle: usize) {}
}

pub struct UBOStorage<T: RALBackend, U> {
  storage: Vec<U>,
  dirty: bool,
  // dirty_mark: Vec<bool>,
  gpu: Option<T::UniformBuffer>,
}

impl<T: RALBackend, U> UBOStorage<T, U> {
  pub fn new() -> Self {
    Self {
      storage: Vec::new(),
      dirty: true,
      gpu: None,
    }
  }
}
