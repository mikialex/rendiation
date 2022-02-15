use std::{
  cell::RefCell,
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  rc::Rc,
};

use crate::*;

pub trait BindableResourceView {
  fn as_bindable(&self) -> wgpu::BindingResource;
}

pub struct PlaceholderBindgroup;

impl PlaceholderBindgroup {
  pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "PlaceholderBindgroup".into(),
      entries: &[],
    })
  }
}

pub struct BindGroupObject {
  bg: Rc<wgpu::BindGroup>,
}

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

pub trait BindProvider: BindableResourceView {
  fn view_id(&self) -> usize;
  fn add_bind_record(&self, record: BindGroupCacheInvalidation);
}

pub struct ShaderBindingResult {
  bindings: Vec<BindGroupObject>,
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

  pub fn setup_uniform<T>(&mut self, group: usize, item: &ResourceViewRc<T>)
  where
    T: Resource,
    T::View: BindableResourceView,
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

      pass.set_bind_group_owned(group_index as u32, bindgroup, &[]);
    }
  }
}
