use std::{
  any::{Any, TypeId},
  cell::UnsafeCell,
  collections::HashMap,
  rc::Rc,
};

pub struct BindGroupLayoutManager {
  cache: UnsafeCell<HashMap<TypeId, Rc<wgpu::BindGroupLayout>>>,
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
  ) -> &Rc<wgpu::BindGroupLayout> {
    let map = self.cache.get();
    let map = unsafe { &mut *map };
    map
      .entry(TypeId::of::<T>())
      .or_insert_with(|| Rc::new(T::layout(device)))
  }
}

impl Default for BindGroupLayoutManager {
  fn default() -> Self {
    Self::new()
  }
}

/// The pipeline cache container abstraction
///
/// To get a cached pipeline, the common idea is to hashing the relevant state
/// and visit a hashmap. In this case, the hashmap is the pipeline cache container.
/// But to maximize performance, some case user just don't need hash if they know
/// enough information about the cached pipeline. For example only cache the pipeline
/// variant by primitive topology
///
/// This trait abstract the variant key to cached pipeline get and create logic
/// and user can compose their key and container to compose the cache container behavior
/// precisely
pub trait PipelineVariantContainer: Default {
  type Key;
  fn request(&mut self, variant: &Self::Key, creator: impl FnOnce() -> wgpu::RenderPipeline);

  fn retrieve(&self, variant: &Self::Key) -> &Rc<wgpu::RenderPipeline>;
}

pub enum PipelineUnit {
  Created(Rc<wgpu::RenderPipeline>),
  Empty,
}
impl Default for PipelineUnit {
  fn default() -> Self {
    PipelineUnit::Empty
  }
}

impl PipelineVariantContainer for PipelineUnit {
  type Key = ();
  fn request(&mut self, _variant: &Self::Key, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    if let PipelineUnit::Empty = self {
      *self = PipelineUnit::Created(Rc::new(creator()));
    }
  }
  fn retrieve(&self, _variant: &Self::Key) -> &Rc<wgpu::RenderPipeline> {
    match self {
      PipelineUnit::Created(p) => p,
      PipelineUnit::Empty => unreachable!(),
    }
  }
}

pub struct TopologyPipelineVariant<T> {
  pipelines: [Option<T>; 5],
}

impl<T> Default for TopologyPipelineVariant<T> {
  fn default() -> Self {
    Self {
      pipelines: [None, None, None, None, None],
    }
  }
}

pub struct PipelineVariantKey<T, K> {
  pub current: K,
  pub inner: T,
}

pub trait PipelineVariantKeyBuilder: Sized {
  fn key_with<K>(self, current: K) -> PipelineVariantKey<Self, K> {
    PipelineVariantKey {
      current,
      inner: self,
    }
  }
}
impl<T> PipelineVariantKeyBuilder for T {}

impl<T: PipelineVariantContainer> PipelineVariantContainer for TopologyPipelineVariant<T> {
  type Key = PipelineVariantKey<T::Key, wgpu::PrimitiveTopology>;
  fn request(&mut self, variant: &Self::Key, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    self.pipelines[variant.current as usize]
      .get_or_insert_with(Default::default)
      .request(&variant.inner, creator);
  }

  fn retrieve(&self, variant: &Self::Key) -> &Rc<wgpu::RenderPipeline> {
    self.pipelines[variant.current as usize]
      .as_ref()
      .unwrap()
      .retrieve(&variant.inner)
  }
}

pub struct PipelineResourceManager {
  pub cache: HashMap<TypeId, Box<dyn Any>>,
}

pub trait PipelineRequester: Any {
  type Container: PipelineVariantContainer;
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  pub fn get_cache_mut<M: PipelineRequester>(&mut self) -> &mut M::Container {
    self
      .cache
      .entry(TypeId::of::<M>())
      .or_insert_with(|| Box::new(M::Container::default()))
      .downcast_mut::<M::Container>()
      .unwrap()
  }

  pub fn get_cache<M: PipelineRequester>(&self) -> &M::Container {
    self
      .cache
      .get(&TypeId::of::<M>())
      .unwrap()
      .downcast_ref::<M::Container>()
      .unwrap()
  }
}

impl Default for PipelineResourceManager {
  fn default() -> Self {
    Self::new()
  }
}
