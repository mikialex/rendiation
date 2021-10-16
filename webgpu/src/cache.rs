use std::{
  any::{Any, TypeId},
  cell::UnsafeCell,
  collections::HashMap,
};

pub struct BindGroupLayoutManager {
  cache: UnsafeCell<HashMap<TypeId, wgpu::BindGroupLayout>>,
}

pub trait BindGroupLayoutProvider {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;
}

impl BindGroupLayoutManager {
  pub fn new() -> Self {
    Self {
      cache: UnsafeCell::new(HashMap::new()),
    }
  }

  pub fn retrieve<T: BindGroupLayoutProvider + Any>(
    &self,
    device: &wgpu::Device,
  ) -> &wgpu::BindGroupLayout {
    let map = self.cache.get();
    let map = unsafe { &mut *map };
    map
      .entry(TypeId::of::<T>())
      .or_insert_with(|| T::layout(device))
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
