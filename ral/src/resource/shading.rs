use super::ShaderBindableResourceManager;
use crate::{RALBackend, ShadingHandle, ShadingProvider};
use arena::{Arena, Handle};
use std::{any::Any, collections::HashSet};

pub struct ShadingManager<R: RALBackend> {
  storage: Arena<Box<dyn BindgroupStorageTrait<R>>>,
  modified: HashSet<Handle<Box<dyn BindgroupStorageTrait<R>>>>,
}

impl<R: RALBackend> ShadingManager<R> {
  pub fn new() -> Self {
    Self {
      storage: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn maintain_gpu(
    &mut self,
    renderer: &R::Renderer,
    resources: &Box<ShaderBindableResourceManager<R>>,
  ) {
    let storage = &mut self.storage;
    self.modified.drain().for_each(|d| {
      storage.get_mut(d).map(|bp| {
        bp.maintain_gpu(renderer, resources);
      });
    })
  }

  pub fn get_gpu<T: ShadingProvider<R>>(&self, handle: ShadingHandle<R, T>) -> &R::Shading {
    let handle = unsafe { handle.cast_type() };
    self.storage.get(handle).unwrap().get_gpu()
  }

  pub fn add_shading<T: ShadingProvider<R>>(&mut self, bindgroup: T) -> ShadingHandle<R, T> {
    let pair = ShadingPair {
      data: bindgroup,
      gpu: None,
    };
    let handle = self.storage.insert(Box::new(pair));
    self.modified.insert(handle);
    unsafe { handle.cast_type() }
  }

  pub fn update_shading<T: ShadingProvider<R>>(&mut self, handle: ShadingHandle<R, T>) -> &mut T {
    let handle = unsafe { handle.cast_type() };
    self.modified.insert(handle);
    let pair = self.storage.get_mut(handle).unwrap();
    pair
      .as_any_mut()
      .downcast_mut::<ShadingPair<R, T>>()
      .unwrap()
      .update()
  }

  pub fn delete_shading<T: ShadingProvider<R>>(&mut self, handle: ShadingHandle<R, T>) {
    let handle = unsafe { handle.cast_type() };
    self.modified.remove(&handle);
    self.storage.remove(handle);
  }
}

trait BindgroupStorageTrait<R: RALBackend>: Any {
  fn maintain_gpu(
    &mut self,
    renderer: &R::Renderer,
    resources: &Box<ShaderBindableResourceManager<R>>,
  );
  fn get_gpu(&self) -> &R::Shading;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<R: RALBackend, T: ShadingProvider<R>> BindgroupStorageTrait<R> for ShadingPair<R, T> {
  fn maintain_gpu<'a>(
    &mut self,
    renderer: &R::Renderer,
    resources: &Box<ShaderBindableResourceManager<R>>,
  ) {
    self.gpu = Some(self.data.create_shading(renderer, resources.as_any()));
  }
  fn get_gpu(&self) -> &R::Shading {
    self.gpu.as_ref().unwrap()
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct ShadingPair<R: RALBackend, T: ShadingProvider<R>> {
  data: T,
  gpu: Option<R::Shading>,
}

impl<R: RALBackend, T: ShadingProvider<R>> ShadingPair<R, T> {
  fn update(&mut self) -> &mut T {
    &mut self.data
  }
}
