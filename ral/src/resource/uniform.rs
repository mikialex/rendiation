use crate::{UniformBufferRef, UniformHandle, RAL};
use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  marker::PhantomData,
  ops::Range,
};

pub struct UBOManager<T: RAL> {
  data: HashMap<TypeId, Box<dyn UBOStorageTrait<T>>>,
  modified: HashSet<TypeId>,
}

impl<T: RAL> UBOManager<T> {
  pub fn new() -> Self {
    Self {
      data: HashMap::new(),
      modified: HashSet::new(),
    }
  }

  pub fn add<U: 'static>(&mut self, value: U) -> UniformHandle<T, U> {
    UniformHandle {
      index: self.insert(value),
      phantom: PhantomData,
      phantom2: PhantomData,
    }
  }

  pub fn get_storage_or_create<U: 'static>(&mut self) -> &mut UBOStorage<T, U> {
    let id = TypeId::of::<U>();
    let modified = &mut self.modified;
    self
      .data
      .entry(id)
      .or_insert_with(|| {
        modified.insert(id);
        Box::new(UBOStorage::<T, U>::new())
      })
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

  fn get_gpu_with_range<U: 'static>(&self, handle: usize) -> (&T::UniformBuffer, Range<u64>) {
    (
      self.get_storage_should_ok::<U>().get_gpu(),
      handle as u64..(handle + 1) as u64,
    )
  }

  pub fn get_uniform_gpu<U: 'static>(&self, handle: UniformHandle<T, U>) -> UniformBufferRef<T, U> {
    UniformBufferRef {
      ty: PhantomData,
      gpu: self.get_gpu_with_range::<U>(handle.index),
      data: self.get_storage_should_ok::<U>().get_data(handle.index),
    }
  }

  pub fn delete<U: 'static>(&mut self, handle: usize) {
    self.get_storage_or_create::<U>().delete(handle);
  }

  fn insert<U: 'static>(&mut self, value: U) -> usize {
    self.notify_modified::<U>();
    self.get_storage_or_create::<U>().insert(value)
  }

  pub fn notify_modified<U: 'static>(&mut self) {
    self.modified.insert(TypeId::of::<U>());
  }

  pub fn update<U: 'static>(&mut self, handle: UniformHandle<T, U>, new_value: U) {
    self.notify_modified::<U>();
    self
      .get_storage_or_create::<U>()
      .update(handle.index, new_value);
  }

  pub fn mutate<U: 'static>(&mut self, handle: UniformHandle<T, U>) -> &mut U {
    self.notify_modified::<U>();
    self.get_storage_or_create::<U>().mutate(handle.index)
  }

  pub fn get_data<U: 'static>(&self, handle: UniformHandle<T, U>) -> &U {
    self.get_storage_should_ok::<U>().get_data(handle.index)
  }
}

trait UBOStorageTrait<T: RAL>: Any {
  fn maintain_gpu(&mut self, _renderer: &mut T::Renderer);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: RAL, U: 'static> UBOStorageTrait<T> for UBOStorage<T, U> {
  fn maintain_gpu(&mut self, renderer: &mut T::Renderer) {
    if self.dirty {
      let ptr = self.storage.as_ptr();
      let data = unsafe {
        let ptr = std::mem::transmute(ptr);
        std::slice::from_raw_parts::<u8>(ptr, self.storage.len() * std::mem::size_of::<U>())
      };

      if let Some(gpu) = &mut self.gpu {
        T::update_uniform_buffer(renderer, gpu, data, 0..self.storage.len());
      } else {
        self.gpu = Some(T::create_uniform_buffer(renderer, data))
      }
    }
    self.dirty = false;
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct UBOStorage<T: RAL, U> {
  storage: Vec<U>,
  dirty: bool,
  // dirty_mark: Vec<bool>,
  gpu: Option<T::UniformBuffer>,
}

impl<T: RAL, U> UBOStorage<T, U> {
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

  fn mutate(&mut self, handle: usize) -> &mut U {
    &mut self.storage[handle]
  }

  fn get_gpu(&self) -> &T::UniformBuffer {
    self.gpu.as_ref().unwrap()
  }

  fn get_data(&self, handle: usize) -> &U {
    &self.storage[handle]
  }
}
