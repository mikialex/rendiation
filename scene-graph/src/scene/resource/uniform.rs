use rendiation_ral::RALBackend;
use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  marker::PhantomData,
};

pub struct UBOManager<T: RALBackend> {
  data: HashMap<TypeId, Box<dyn Any>>,
  modified: HashSet<TypeId>,
  phantom: PhantomData<T>,
}

impl<T: RALBackend> UBOManager<T> {
  pub fn new() -> Self {
    Self {
      data: HashMap::new(),
      modified: HashSet::new(),
      phantom: PhantomData,
    }
  }

  pub fn get_storage<U: 'static>(&mut self) -> &mut UBOStorage<T, U> {
    self
      .data
      .get_mut(&TypeId::of::<U>())
      .unwrap()
      .downcast_mut::<UBOStorage<T, U>>()
      .unwrap()
  }

  pub fn delete<U: 'static>(&mut self, handle: usize) {
    self.get_storage::<U>().delete(handle);
  }

  pub fn insert<U: 'static>(&mut self, value: U) {
    self.get_storage::<U>().insert(value);
  }

  pub fn update<U: 'static>(&mut self, handle: usize, new_value: U) {
    self.get_storage::<U>().update(handle, new_value);
  }
}

pub struct UBOStorage<T: RALBackend, U> {
  storage: Vec<U>,
  dirty: bool,
  // dirty_mark: Vec<bool>,
  gpu: Option<T::UniformBuffer>,
}

impl<T: RALBackend, U> UBOStorage<T, U> {
  fn new() -> Self {
    Self {
      storage: Vec::new(),
      dirty: true,
      gpu: None,
    }
  }

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
