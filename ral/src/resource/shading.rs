use crate::{
  AnyPlaceHolder, BindGroupManager, GeometryProvider, ResourceManager, ShaderGeometryInfo,
  ShadingCreator, ShadingHandle, ShadingProvider, RAL,
};
use arena::{Arena, Handle};
use std::{any::Any, any::TypeId, collections::HashMap, collections::HashSet, rc::Rc};

pub struct ShadingManager<R: RAL> {
  gpu: HashMap<TypeId, Rc<R::Shading>>,
  storage: Arena<Box<dyn ShadingStorageTrait<R>>>,
  modified: HashSet<Handle<Box<dyn ShadingStorageTrait<R>>>>,
}

impl<R: RAL> ShadingManager<R> {
  pub fn new() -> Self {
    Self {
      gpu: HashMap::new(),
      storage: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn maintain_gpu(&mut self, _renderer: &R::Renderer, _resources: &BindGroupManager<R>) {
    self.modified.clear();
  }

  pub fn get_shading<T: ShadingProvider<R>>(
    &self,
    handle: ShadingHandle<R, T>,
  ) -> &ShadingPair<R, T> {
    let handle = unsafe { handle.cast_type() };
    self
      .storage
      .get(handle)
      .unwrap()
      .as_any()
      .downcast_ref()
      .unwrap()
  }

  pub fn get_shading_boxed<T: ShadingProvider<R>>(
    &self,
    handle: ShadingHandle<R, T>,
  ) -> &dyn ShadingStorageTrait<R> {
    let handle = unsafe { handle.cast_type() };
    self.storage.get(handle).unwrap().as_ref()
  }

  pub fn add_shading<T: ShadingCreator<R>>(
    &mut self,
    shading: T::Instance,
    renderer: &mut R::Renderer,
  ) -> ShadingHandle<R, T> {
    let type_id = TypeId::of::<T>();
    let gpu = self
      .gpu
      .entry(type_id)
      .or_insert_with(|| Rc::new(T::create_shader(&shading, renderer)))
      .clone();
    let pair = ShadingPair::<R, T> { data: shading, gpu };
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
    println!("delete");
    let handle = unsafe { handle.cast_type() };
    self.modified.remove(&handle);
    self.storage.remove(handle);
  }
}

pub trait ShadingStorageTrait<R: RAL>: Any {
  fn get_gpu(&self) -> &R::Shading;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn shading_provider_as_any(&self) -> &dyn Any;
  fn apply(&self, render_pass: &mut R::RenderPass, resources: &ResourceManager<R>);
}

impl<R: RAL, T: ShadingProvider<R>> ShadingStorageTrait<R> for ShadingPair<R, T> {
  fn get_gpu(&self) -> &R::Shading {
    &self.gpu
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn shading_provider_as_any(&self) -> &dyn Any {
    &self.data
  }
  fn apply(&self, render_pass: &mut R::RenderPass, resources: &ResourceManager<R>) {
    T::apply(&self.data, self.get_gpu(), render_pass, resources);
  }
}

pub struct ShadingPair<R: RAL, T: ShadingProvider<R>> {
  pub data: T::Instance,
  pub gpu: Rc<R::Shading>,
}

impl<R: RAL, T: ShadingProvider<R>> ShadingPair<R, T> {
  fn update(&mut self) -> &mut T::Instance {
    &mut self.data
  }
}

pub struct AnyPlaceHolderShadingProviderInstance;
impl<T: RAL> ShadingProvider<T> for AnyPlaceHolder {
  type Instance = AnyPlaceHolderShadingProviderInstance;
  fn apply(
    _instance: &Self::Instance,
    _gpu_shading: &T::Shading,
    _render_pass: &mut T::RenderPass,
    _resources: &ResourceManager<T>,
  ) {
    unreachable!()
  }
}

impl ShaderGeometryInfo for AnyPlaceHolder {
  type Geometry = AnyGeometryProvider;
}

pub struct AnyGeometryProvider;
impl GeometryProvider for AnyGeometryProvider {}
