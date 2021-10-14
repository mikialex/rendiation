use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

pub struct BindGroupLayoutManager {
  pub cache: HashMap<TypeId, wgpu::BindGroupLayout>,
}

impl BindGroupLayoutManager {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  pub fn register<T: Any>(&mut self, layout: wgpu::BindGroupLayout) {
    self.cache.insert(TypeId::of::<T>(), layout);
  }

  pub fn retrieve<T: Any>(&self) -> &wgpu::BindGroupLayout {
    self.cache.get(&TypeId::of::<T>()).unwrap()
  }
}

impl Default for BindGroupLayoutManager {
  fn default() -> Self {
    Self::new()
  }
}

pub struct PipelineResourceManager {
  pub cache: HashMap<TypeId, Box<dyn Any>>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  pub fn get_cache_mut<M: Any, C: Any + Default>(&mut self) -> &mut C {
    self
      .cache
      .entry(TypeId::of::<M>())
      .or_insert_with(|| Box::new(C::default()))
      .downcast_mut::<C>()
      .unwrap()
  }

  pub fn get_cache<M: Any, C: Any>(&self) -> &C {
    self
      .cache
      .get(&TypeId::of::<M>())
      .unwrap()
      .downcast_ref::<C>()
      .unwrap()
  }
}

impl Default for PipelineResourceManager {
  fn default() -> Self {
    Self::new()
  }
}
