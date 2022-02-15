use std::{
  any::Any,
  cell::RefCell,
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  rc::Rc,
};

use rendiation_webgpu::{GPURenderPass, GPURenderPipeline};
use shadergraph::SemanticShaderUniform;

pub trait ShaderBindingProvider {
  fn setup_binding(&self, builder: &mut BindingBuilder);
}

#[derive(Clone)]
pub struct BindGroupCache {
  cache: Rc<RefCell<HashMap<u64, Rc<wgpu::BindGroup>>>>,
}

#[derive(Clone)]
pub struct BindGroupLayoutCache {
  cache: Rc<RefCell<HashMap<u64, Rc<wgpu::BindGroupLayout>>>>,
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

pub struct ResourceRc<T: Resource> {
  inner: Rc<ResourceContainer<T>>,
}

impl<T: Resource> Clone for ResourceRc<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: Resource> ResourceRc<T> {
  pub fn create(&self, desc: T::Descriptor, device: &wgpu::Device) -> Self {
    Self {
      inner: Rc::new(ResourceContainer::create(desc, device)),
    }
  }

  pub fn create_view(
    &self,
    desc: T::ViewDescriptor,
    device: &wgpu::Device,
  ) -> ResourceViewContainer<T> {
    let view = self.inner.resource.create_view(&desc, device);
    ResourceViewContainer {
      resource: self.clone(),
      view,
      guid: todo!(),
      desc,
    }
  }
}

pub struct ResourceViewRc<T: Resource> {
  inner: Rc<ResourceViewContainer<T>>,
}

impl<T: Resource> Clone for ResourceViewRc<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: Resource> BindProvider for ResourceViewRc<T> {
  fn view_id(&self) -> usize {
    self.inner.guid
  }

  fn as_bindable(&self) -> wgpu::BindingResource {
    todo!()
  }

  fn add_bind_record(&self, record: BindGroupCacheInvalidation) {
    self
      .inner
      .resource
      .inner
      .invalidation_tokens
      .borrow_mut()
      .push(record);
  }
}

pub struct ResourceViewContainer<T: Resource> {
  // when resource view is hold, the resource it self should keep existing
  resource: ResourceRc<T>,
  view: T::View,
  guid: usize,
  desc: T::ViewDescriptor,
}

/// store the resource with it's create parameter,
/// and some dropping callbacks
pub struct ResourceContainer<T: Resource> {
  guid: usize,
  resource: T,
  desc: T::Descriptor,
  /// when resource dropped, all referenced bindgroup should drop
  invalidation_tokens: RefCell<Vec<BindGroupCacheInvalidation>>,
}

impl<T: Resource> ResourceContainer<T> {
  pub fn create(desc: T::Descriptor, device: &wgpu::Device) -> Self {
    let resource = T::create_resource(&desc, device);
    Self {
      guid: todo!(),
      resource,
      desc,
      invalidation_tokens: Default::default(),
    }
  }
}

pub trait Resource: 'static {
  type Descriptor;
  type View;
  type ViewDescriptor;

  fn create_resource(des: &Self::Descriptor, device: &wgpu::Device) -> Self;
  fn create_view(&self, des: &Self::ViewDescriptor, device: &wgpu::Device) -> Self::View;
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

pub struct BindingBuilder {
  cache: BindGroupCache,
  items: [Vec<Box<dyn BindProvider>>; 5],
}

impl BindingBuilder {
  pub fn create(cache: &BindGroupCache) -> Self {
    Self {
      cache: cache.clone(),
      items: Default::default(),
    }
  }

  pub fn setup_uniform<T: Resource>(&mut self, group: usize, item: &ResourceViewRc<T>)
  // where
  //   T: SemanticShaderUniform,
  {
    self.items[group].push(Box::new(item.clone()))
  }

  pub fn setup_pass(
    &self,
    pass: &mut GPURenderPass,
    device: &wgpu::Device,
    pipeline: &GPURenderPipeline,
  ) {
    for (group_index, group) in self.items.iter().enumerate() {
      if group.is_empty() {
        pass.set_bind_group_placeholder(group_index as u32);
      }

      // hash
      let mut hasher = DefaultHasher::default();
      group.iter().for_each(|b| {
        b.view_id().hash(&mut hasher);
        // todo hash bind ty
        // hash ty could only hash the bindgroup layout guid
      });
      let hash = hasher.finish();

      let mut cache = self.cache.cache.borrow_mut();

      let bindgroup = cache.entry(hash).or_insert_with(|| {
        // build bindgroup and cache and return
        let entries: Vec<_> = group
          .iter()
          .enumerate()
          .map(|(i, item)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: item.as_bindable(),
          })
          .collect();

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
          label: None,
          layout: &pipeline.bg_layouts[group_index],
          entries: &entries,
        });
        Rc::new(bindgroup)
      });

      pass.set_bind_group_owned(group_index as u32, &bindgroup, &[]);
    }
  }
}

pub trait BindProvider {
  fn view_id(&self) -> usize;
  fn as_bindable(&self) -> wgpu::BindingResource;
  fn add_bind_record(&self, record: BindGroupCacheInvalidation);
}

pub struct ShaderBindingResult {
  bindings: Vec<BindGroupObject>,
}
