use std::{
  any::{Any, TypeId},
  cell::RefCell,
  collections::{hash_map::DefaultHasher, HashMap},
  hash::Hasher,
  rc::Rc,
};

use crate::PipelineBuilder;

#[derive(Default)]
pub struct SamplerCache<T> {
  cache: RefCell<HashMap<T, Rc<wgpu::Sampler>>>,
}

impl<T> SamplerCache<T>
where
  T: Eq + std::hash::Hash + Into<wgpu::SamplerDescriptor<'static>> + Clone,
{
  pub fn retrieve(&self, device: &wgpu::Device, desc: &T) -> Rc<wgpu::Sampler> {
    let mut map = self.cache.borrow_mut();
    map
      .entry(desc.clone()) // todo optimize move
      .or_insert_with(|| Rc::new(device.create_sampler(&desc.clone().into())))
      .clone()
  }
}

#[derive(Default)]
pub struct BindGroupLayoutCache {
  cache: RefCell<HashMap<TypeId, Rc<wgpu::BindGroupLayout>>>,
}

pub trait BindGroupLayoutProvider: 'static {
  fn bind_preference() -> usize;
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;
  fn gen_shader_header(group: usize) -> String;
  fn register_uniform_struct_declare(builder: &mut PipelineBuilder);
}

impl BindGroupLayoutCache {
  pub fn retrieve<T: BindGroupLayoutProvider + Any>(
    &self,
    device: &wgpu::Device,
  ) -> Rc<wgpu::BindGroupLayout> {
    let mut map = self.cache.borrow_mut();
    map
      .entry(TypeId::of::<T>())
      .or_insert_with(|| Rc::new(T::layout(device)))
      .clone()
  }
}

#[derive(Default)]
pub struct PipelineResourceCache {
  pub cache: HashMap<u64, Rc<wgpu::RenderPipeline>>,
}

#[derive(Default)]
pub struct PipelineHasher {
  hasher: DefaultHasher,
}

impl std::hash::Hasher for PipelineHasher {
  fn finish(&self) -> u64 {
    self.hasher.finish()
  }

  fn write(&mut self, bytes: &[u8]) {
    self.hasher.write(bytes)
  }
}

impl PipelineResourceCache {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  pub fn get_or_insert_with(
    &mut self,
    hasher: PipelineHasher,
    creator: impl FnOnce() -> wgpu::RenderPipeline,
  ) -> &Rc<wgpu::RenderPipeline> {
    let key = hasher.finish();
    self.cache.entry(key).or_insert_with(|| Rc::new(creator()))
  }
}
