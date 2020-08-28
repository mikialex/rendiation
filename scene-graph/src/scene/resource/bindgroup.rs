use arena::{Arena, Handle};
use rendiation_ral::{BindGroupProvider, RALBackend};
use std::{any::Any, collections::HashSet};

pub struct BindGroupManager<R: RALBackend> {
  storage: Arena<Box<dyn BindgroupStorageTrait<R>>>,
  modified: HashSet<Handle<Box<dyn BindgroupStorageTrait<R>>>>,
}

impl<R: RALBackend> BindGroupManager<R> {
  pub fn new() -> Self {
    Self {
      storage: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn maintain_gpu(&mut self, renderer: &R::Renderer) {
    let storage = &mut self.storage;
    self.modified.drain().for_each(|d| {
      storage.get_mut(d).map(|bp| {
        bp.maintain_gpu(renderer);
      });
    })
  }

  pub fn get_gpu<T: BindGroupProvider<R>>(
    &self,
    handle: Handle<BindgroupPair<R, T>>,
  ) -> &R::BindGroup {
    let handle = unsafe { handle.cast_type() };
    self.storage.get(handle).unwrap().get_gpu()
  }

  pub fn add_bindgroup<T: BindGroupProvider<R>>(
    &mut self,
    bindgroup: T,
  ) -> Handle<BindgroupPair<R, T>> {
    let pair = BindgroupPair {
      data: bindgroup,
      gpu: None,
    };
    let handle = self.storage.insert(Box::new(pair));
    self.modified.insert(handle);
    unsafe { handle.cast_type() }
  }

  pub fn update_bindgroup<T: BindGroupProvider<R>>(
    &mut self,
    handle: Handle<BindgroupPair<R, T>>,
  ) -> &mut T {
    let handle = unsafe { handle.cast_type() };
    self.modified.insert(handle);
    let pair = self.storage.get_mut(handle).unwrap();
    pair
      .as_any_mut()
      .downcast_mut::<BindgroupPair<R, T>>()
      .unwrap()
      .update()
  }

  pub fn delete_bindgroup<T: BindGroupProvider<R>>(&mut self, handle: Handle<BindgroupPair<R, T>>) {
    let handle = unsafe { handle.cast_type() };
    self.modified.remove(&handle);
    self.storage.remove(handle);
  }
}

trait BindgroupStorageTrait<R: RALBackend>: Any {
  fn maintain_gpu(&mut self, renderer: &R::Renderer);
  fn get_gpu(&self) -> &R::BindGroup;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<R: RALBackend, T: BindGroupProvider<R>> BindgroupStorageTrait<R> for BindgroupPair<R, T> {
  fn maintain_gpu(&mut self, renderer: &R::Renderer) {
    self.gpu = Some(self.data.create_bindgroup(renderer));
  }
  fn get_gpu(&self) -> &R::BindGroup {
    self.gpu.as_ref().unwrap()
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct BindgroupPair<R: RALBackend, T: BindGroupProvider<R>> {
  data: T,
  gpu: Option<R::BindGroup>,
}

impl<R: RALBackend, T: BindGroupProvider<R>> BindgroupPair<R, T> {
  fn update(&mut self) -> &mut T {
    &mut self.data
  }
}
