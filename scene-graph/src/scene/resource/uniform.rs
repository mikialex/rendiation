use crate::ResourceManager;
use rendiation_ral::*;
use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  marker::PhantomData,
  ops::Range,
};

pub struct UniformHandle<U> {
  index: usize,
  phantom: PhantomData<U>,
}

impl<T> Clone for UniformHandle<T> {
  fn clone(&self) -> Self {
      *self
  }
}
impl<T> Copy for UniformHandle<T> { }

/// uniform buffer
impl<T: RALBackend> ResourceManager<T> {
  pub fn add_uniform<U: 'static>(&mut self, value: U) -> UniformHandle<U> {
    UniformHandle {
      index: self.uniform_buffers.insert(value),
      phantom: PhantomData,
    }
  }

  pub fn update_uniform<U: 'static>(&mut self, handle: UniformHandle<U>, new_value: U) {
    self.uniform_buffers.update(handle.index, new_value);
  }

  pub fn get_uniform_gpu<U: 'static>(&self, handle: UniformHandle<U>) -> UniformBufferRef<T, U> {
    UniformBufferRef {
      ty: PhantomData,
      data: self.uniform_buffers.get_gpu_with_range::<U>(handle.index),
    }
  }

  pub fn delete_uniform<U: 'static>(&mut self, handle: UniformHandle<U>) {
    self.uniform_buffers.delete::<U>(handle.index);
  }
}

pub struct UBOManager<T: RALBackend> {
  data: HashMap<TypeId, Box<dyn UBOStorageTrait<T>>>,
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
      .entry(TypeId::of::<U>())
      .or_insert_with(|| Box::new(UBOStorage::<T, U>::new()))
      .as_any_mut()
      .downcast_mut::<UBOStorage<T, U>>()
      .unwrap()
  }
  pub fn get_storage_should_ok<U: 'static>(&self) -> &UBOStorage<T, U> {
    self
      .data
      .get(&TypeId::of::<U>())
      .unwrap()
      .as_any()
      .downcast_ref::<UBOStorage<T, U>>()
      .unwrap()
  }

  pub fn maintain_gpu(&mut self, renderer: &mut T::Renderer) {
    let data = &mut self.data;
    self.modified.drain().for_each(|ty| {
      data.get_mut(&ty).map(|storage| {
        storage.maintain_gpu(renderer);
      });
    });
  }

  pub fn get_gpu_with_range<U: 'static>(&self, handle: usize) -> (&T::UniformBuffer, Range<u64>) {
    let stride = std::mem::size_of::<U>();
    (
      self.get_storage_should_ok::<U>().get_gpu(),
      (handle * stride) as u64..((handle + 1) * stride) as u64,
    )
  }

  pub fn delete<U: 'static>(&mut self, handle: usize) {
    self.get_storage::<U>().delete(handle);
  }

  pub fn insert<U: 'static>(&mut self, value: U) -> usize {
    self.get_storage::<U>().insert(value)
  }

  pub fn update<U: 'static>(&mut self, handle: usize, new_value: U) {
    self.get_storage::<U>().update(handle, new_value);
  }
}

trait UBOStorageTrait<T: RALBackend>: Any {
  fn maintain_gpu(&mut self, _renderer: &mut T::Renderer);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: RALBackend, U: 'static> UBOStorageTrait<T> for UBOStorage<T, U> {
  fn maintain_gpu(&mut self, renderer: &mut T::Renderer) {
    if self.dirty {
      let data = self.storage.as_slice();
      let data = unsafe { std::mem::transmute(data) };
      self.gpu = Some(T::create_uniform_buffer(renderer, data))
    }
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
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

  fn get_gpu(&self) -> &T::UniformBuffer {
    self.gpu.as_ref().unwrap()
  }
}
