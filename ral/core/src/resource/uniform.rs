use crate::{
  AnyBindGroupType, BindGroupHandle, BindGroupManager, UniformBufferRef, UniformHandle, RAL,
};
use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  marker::PhantomData,
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

  pub fn maintain_gpu(&mut self, renderer: &mut T::Renderer, bgm: &mut BindGroupManager<T>) {
    let data = &mut self.data;
    self.modified.drain().for_each(|ty| {
      data.get_mut(&ty).map(|storage| {
        storage.maintain_gpu(renderer, bgm);
      });
    });
  }

  pub fn get_uniform_gpu<U: 'static>(&self, handle: UniformHandle<T, U>) -> UniformBufferRef<T, U> {
    UniformBufferRef {
      ty: PhantomData,
      gpu: (
        self.get_storage_should_ok::<U>().get_gpu(handle.index),
        0..1,
      ),
      data: self.get_storage_should_ok::<U>().get_data(handle.index),
    }
  }

  pub fn delete<U: 'static>(&mut self, handle: UniformHandle<T, U>) {
    self.get_storage_or_create::<U>().delete(handle.index);
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
  fn maintain_gpu(&mut self, renderer: &mut T::Renderer, bgm: &mut BindGroupManager<T>);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: RAL, U: 'static> UBOStorageTrait<T> for UBOStorage<T, U> {
  #[allow(clippy::transmute_ptr_to_ptr)]
  fn maintain_gpu(&mut self, renderer: &mut T::Renderer, _bgm: &mut BindGroupManager<T>) {
    let storage = &self.storage;
    let gpu = &mut self.gpu;
    self.dirty_set.drain().for_each(|handle| {
      let data = storage.get(handle..handle + 1).unwrap();
      let data = unsafe { std::mem::transmute(data) };
      gpu[handle] = Some(T::create_uniform_buffer(renderer, data));
    });
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

/// The reason we not use array of struct is we want storage stored in continues memory for best locality
pub struct UBOStorage<T: RAL, U> {
  storage: Vec<U>,
  // _bindgroup_referenced: Vec<HashSet<BindGroupHandle<T, AnyBindGroupType>>>,
  gpu: Vec<Option<T::UniformBuffer>>,
  dirty_set: HashSet<usize>,
}

impl<T: RAL, U> UBOStorage<T, U> {
  fn new() -> Self {
    Self {
      // _bindgroup_referenced: Vec::new(),
      storage: Vec::new(),
      gpu: Vec::new(),
      dirty_set: HashSet::new(),
    }
  }

  fn insert(&mut self, value: U) -> usize {
    let result = self.storage.len();
    self.storage.push(value);
    self.gpu.push(None);
    self.dirty_set.insert(result);
    result
  }

  fn delete(&mut self, handle: usize) {
    self.storage.swap_remove(handle);
    self.dirty_set.remove(&handle);
  }

  fn update(&mut self, handle: usize, new_value: U) {
    self.storage[handle] = new_value;
    self.dirty_set.insert(handle);
  }

  fn mutate(&mut self, handle: usize) -> &mut U {
    self.dirty_set.insert(handle);
    &mut self.storage[handle]
  }

  fn get_gpu(&self, index: usize) -> &T::UniformBuffer {
    self.gpu.get(index).unwrap().as_ref().unwrap()
  }

  fn get_data(&self, handle: usize) -> &U {
    &self.storage[handle]
  }

  pub fn add_reference(&mut self, _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>) {
    // self.bindgroup_referenced.insert(bindgroup_handle);
  }
  pub fn remove_reference(&mut self, _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>) {
    // self.bindgroup_referenced.remove(&bindgroup_handle);
  }
}
