use crate::{AnyPlaceHolder, BindGroupManager, RALBackend, ShadingHandle, ShadingProvider};
use arena::{Arena, Handle};
use std::{any::Any, collections::HashSet};

pub struct AnyPlaceHolderShaderProviderInstance;
impl<T: RALBackend> ShadingProvider<T> for AnyPlaceHolder {
  type Instance = AnyPlaceHolderShaderProviderInstance;
  fn apply(
    _instance: &Self::Instance,
    _render_pass: &mut T::RenderPass,
    _gpu_shading: &T::Shading,
    _resources: &BindGroupManager<T>,
  ) {
    unreachable!()
  }
}

pub struct ShadingManager<R: RALBackend> {
  storage: Arena<Box<dyn ShadingStorageTrait<R>>>,
  modified: HashSet<Handle<Box<dyn ShadingStorageTrait<R>>>>,
}

impl<R: RALBackend> ShadingManager<R> {
  pub fn new() -> Self {
    Self {
      storage: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn maintain_gpu(&mut self, _renderer: &R::Renderer, _resources: &BindGroupManager<R>) {
    // let storage = &mut self.storage;
    // self.modified.drain().for_each(|d| {
    //   storage.get_mut(d).map(|bp| {
    //     bp.maintain_gpu(renderer, resources);
    //   });
    // })
  }

  pub fn get_shading<T: ShadingProvider<R>>(
    &self,
    _handle: ShadingHandle<R, T>,
  ) -> &ShadingPair<R, T> {
    todo!()
    // let handle = unsafe { handle.cast_type() };
    // self.storage.get(handle).unwrap()
  }

  pub fn get_shading_boxed(
    &self,
    handle: ShadingHandle<R, AnyPlaceHolder>,
  ) -> &Box<dyn ShadingStorageTrait<R>> {
    let handle = unsafe { handle.cast_type() };
    self.storage.get(handle).unwrap()
  }

  pub fn add_shading<T: ShadingProvider<R>>(
    &mut self,
    bindgroup: T::Instance,
  ) -> ShadingHandle<R, T> {
    let pair = ShadingPair::<R, T> {
      data: bindgroup,
      gpu: None,
    };
    let handle = self.storage.insert(Box::new(pair));
    self.modified.insert(handle);
    unsafe { handle.cast_type() }
  }

  pub fn update_shading<T: ShadingProvider<R>>(
    &mut self,
    handle: ShadingHandle<R, T>,
  ) -> &mut T::Instance {
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

pub trait ShadingStorageTrait<R: RALBackend>: Any {
  // fn maintain_gpu<'a>(&mut self, renderer: &R::Renderer, resources: &BindGroupManager<R>);
  fn get_gpu(&self) -> &R::Shading;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn apply(&self, render_pass: &mut R::RenderPass, resources: &BindGroupManager<R>);
}

impl<R: RALBackend, T: ShadingProvider<R>> ShadingStorageTrait<R> for ShadingPair<R, T> {
  // fn maintain_gpu<'a>(&mut self, renderer: &R::Renderer, resources: &BindGroupManager<R>) {
  //   // self.gpu = Some(self.data.create_shading(renderer, resources));
  // }
  fn get_gpu(&self) -> &R::Shading {
    self.gpu.as_ref().unwrap()
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn apply(&self, render_pass: &mut R::RenderPass, resources: &BindGroupManager<R>) {
    T::apply(&self.data, render_pass, self.get_gpu(), resources);
  }
}

pub struct ShadingPair<R: RALBackend, T: ShadingProvider<R>> {
  data: T::Instance,
  gpu: Option<R::Shading>,
}

impl<R: RALBackend, T: ShadingProvider<R>> ShadingPair<R, T> {
  fn update(&mut self) -> &mut T::Instance {
    &mut self.data
  }
}
