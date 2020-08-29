use super::ResourceManager;
use crate::{RALBackend, ResourceWrap, UniformBufferRef, UniformHandle};
use arena::Handle;
use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  marker::PhantomData,
  ops::Range,
};

/// uniform buffer
impl<T: RALBackend> ResourceManager<T> {
  pub fn add_uniform<U: 'static>(&mut self, value: U) -> UniformHandle<U> {
    UniformHandle {
      index: self.bindable.uniform_buffers.insert(value),
      phantom: PhantomData,
    }
  }

  pub fn update_uniform<U: 'static>(&mut self, handle: UniformHandle<U>, new_value: U) {
    self
      .bindable
      .uniform_buffers
      .update(handle.index, new_value);
  }

  pub fn get_uniform_gpu<U: 'static>(&self, handle: UniformHandle<U>) -> UniformBufferRef<T, U> {
    UniformBufferRef {
      ty: PhantomData,
      data: self
        .bindable
        .uniform_buffers
        .get_gpu_with_range::<U>(handle.index),
    }
  }

  pub fn delete_uniform<U: 'static>(&mut self, handle: UniformHandle<U>) {
    self.bindable.uniform_buffers.delete::<U>(handle.index);
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

  pub fn get_gpu_with_range<U: 'static>(&self, handle: usize) -> (&T::UniformBuffer, Range<u64>) {
    (
      self.get_storage_should_ok::<U>().get_gpu(),
      handle as u64..(handle + 1) as u64,
    )
  }

  pub fn delete<U: 'static>(&mut self, handle: usize) {
    self.get_storage_or_create::<U>().delete(handle);
  }

  pub fn insert<U: 'static>(&mut self, value: U) -> usize {
    self.notify_modified::<U>();
    self.get_storage_or_create::<U>().insert(value)
  }

  pub fn notify_modified<U: 'static>(&mut self) {
    self.modified.insert(TypeId::of::<U>());
  }

  pub fn update<U: 'static>(&mut self, handle: usize, new_value: U) {
    self.notify_modified::<U>();
    self.get_storage_or_create::<U>().update(handle, new_value);
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

pub type UniformValueHandle<T> = Handle<ResourceWrap<<T as RALBackend>::UniformValue>>;

/// uniform values
impl<T: RALBackend> ResourceManager<T> {
  pub fn add_uniform_value(&mut self, gpu: T::UniformValue) -> &mut ResourceWrap<T::UniformValue> {
    ResourceWrap::new_wrap(&mut self.bindable.uniform_values, gpu)
  }

  pub fn get_uniform_value_mut(
    &mut self,
    index: UniformValueHandle<T>,
  ) -> &mut ResourceWrap<T::UniformValue> {
    self.bindable.uniform_values.get_mut(index).unwrap()
  }

  pub fn get_uniform_value(&self, index: UniformValueHandle<T>) -> &ResourceWrap<T::UniformValue> {
    self.bindable.uniform_values.get(index).unwrap()
  }

  pub fn delete_uniform_value(&mut self, index: UniformValueHandle<T>) {
    self.bindable.uniform_values.remove(index);
  }
}
