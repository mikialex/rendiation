use rendiation_ral::RALBackend;
use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
};

pub struct UBOManager<T: RALBackend> {
  data: HashMap<TypeId, Box<dyn UBOStorageTrait<T, dyn Any>>>,
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

trait UBOStorageTrait<T: RALBackend, U> {
  fn insert(&mut self, value: U) -> usize;
  fn delete(&mut self, handle: usize);
  fn update(&mut self, handle: usize, new_value: U);
  fn maintain_gpu(&mut self, renderer: &mut T::Renderer);
  fn get_gpu(&self) -> &T::UniformBuffer;
}

impl<T: RALBackend, U> UBOStorageTrait<T, U> for UBOStorage<T, U> {
  fn insert(&mut self, value: U) -> usize {
    let result = self.storage.len();
    self.storage.push(value);
    self.dirty = true;
    result
  }

  fn delete(&mut self, handle: usize) {
    self.storage.swap_remove(handle);
  }

  fn update(&mut self, handle: usize, new_value: U) {
    self.dirty = true;
    self.storage[handle] = new_value;
  }

  fn maintain_gpu(&mut self, _renderer: &mut T::Renderer) {
    todo!()
  }

  fn get_gpu(&self) -> &T::UniformBuffer {
    self.gpu.as_ref().unwrap()
  }
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
