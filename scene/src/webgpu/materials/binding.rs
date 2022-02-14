use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use shadergraph::SemanticShaderUniform;

pub trait ShaderBindingProvider {
  fn maintain_binding<'a>(&'a self, builder: &mut BindGroupBuilder<'a>);
}

#[derive(Clone)]
pub struct BindGroupCache {
  cache: Rc<RefCell<HashMap<u64, Rc<wgpu::BindGroup>>>>,
}

pub struct BindGroupCacheInvalidation {
  cache_id_to_drop: u64,
  cache: BindGroupCache,
}

impl Drop for BindGroupCacheInvalidation {
  fn drop(&mut self) {
    self.cache.cache.borrow_mut().remove(&self.cache_id_to_drop);
  }
}

#[derive(Clone)]
pub struct ResourceRc<T: Resource> {
  inner: Rc<ResourceContainer<T>>,
}

impl<T: Resource> ResourceRc<T> {
  pub fn create(&self, desc: T::Descriptor) -> Self {
    todo!()
  }

  pub fn create_view(&self, desc: T::ViewDescriptor) -> ResourceViewContainer<T> {
    todo!()
  }
}

pub struct ResourceViewContainer<T: Resource> {
  // when resource view is hold, the resource it self should keep existing
  resource: ResourceRc<T>,
  view: T::View,
  desc: T::ViewDescriptor,
}

/// store the resource with it's create parameter,
/// and some dropping callbacks
pub struct ResourceContainer<T: Resource> {
  resource: T,
  desc: T::Descriptor,
  /// when resource dropped, all referenced bindgroup should drop
  invalidation_tokens: RefCell<Vec<BindGroupCacheInvalidation>>,
}

pub trait Resource {
  type Device;
  type Descriptor;
  type View;
  type ViewDescriptor;
}

pub trait BindingModel {
  type BindGroup;
}

// pub trait BindableResource: Resource {}

pub struct BindGroupObject {
  bg: Rc<wgpu::BindGroup>,
}

pub struct GPUSampler {
  des: wgpu::SamplerDescriptor<'static>,
  sampler_cache: usize,
}

pub struct GPUTexture {
  //
}

pub struct BindGroupBuilder<'a> {
  cache: BindGroupCache,
  items: Vec<Vec<&'a dyn BindProvider>>,
}

pub trait BindProvider {
  fn as_bindable(&self) -> wgpu::BindingResource;
  fn add_bind_record(&self, record: BindGroupCacheInvalidation);
}

impl<'a> BindGroupBuilder<'a> {
  pub fn register_uniform<T>(&mut self, item: &'a T)
  where
    T: SemanticShaderUniform + BindProvider,
  {
    self.items[0].push(item)
  }

  pub fn build(&self) {
    // check if has exist cached bindgroup and return

    // build bindgroup and cache and return
  }
}

pub struct ShaderBindingResult {
  bindings: Vec<BindGroupObject>,
}
