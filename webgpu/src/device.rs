use crate::*;

#[derive(Clone)]
pub struct GPUDevice {
  inner: Rc<GPUDeviceInner>,
}

impl GPUDevice {
  pub fn new(device: gpu::Device) -> Self {
    let inner = GPUDeviceInner {
      device,
      sampler_cache: Default::default(),
      bindgroup_cache: Default::default(),
      bindgroup_layout_cache: Default::default(),
      pipeline_cache: Default::default(),
    };

    Self {
      inner: Rc::new(inner),
    }
  }

  pub(crate) fn create_and_cache_sampler(
    &self,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> RawSampler {
    self.inner.sampler_cache.retrieve(&self.inner.device, desc)
  }

  pub fn get_or_cache_create_render_pipeline(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(&Self) -> GPURenderPipeline,
  ) -> GPURenderPipeline {
    self
      .inner
      .pipeline_cache
      .get_or_insert_with(hasher, || creator(self))
  }

  pub fn create_and_cache_bindgroup_layout(
    &self,
    layouts: &[BindGroupLayoutEntry],
  ) -> GPUBindGroupLayout {
    let mut hasher = DefaultHasher::default();
    layouts.hash(&mut hasher);
    let key = hasher.finish();

    self
      .inner
      .bindgroup_layout_cache
      .cache
      .borrow_mut()
      .entry(key)
      .or_insert_with(|| {
        let inner = self.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
          label: None,
          entries: layouts,
        });
        GPUBindGroupLayout {
          inner: Rc::new(inner),
          cache_id: key,
        }
      })
      .clone()
  }

  pub fn create_binding_builder(&self) -> BindingBuilder {
    BindingBuilder::create(&self.inner.bindgroup_cache)
  }
}

struct GPUDeviceInner {
  device: gpu::Device,
  sampler_cache: SamplerCache,
  bindgroup_cache: BindGroupCache,
  bindgroup_layout_cache: BindGroupLayoutCache,
  pipeline_cache: RenderPipelineCache,
}

impl Deref for GPUDevice {
  type Target = gpu::Device;

  fn deref(&self) -> &Self::Target {
    &self.inner.device
  }
}

pub type RawSampler = Rc<gpu::Sampler>;
#[derive(Default)]
pub struct SamplerCache {
  cache: RefCell<HashMap<GPUSamplerDescriptor, RawSampler>>,
}

impl SamplerCache {
  pub fn retrieve(
    &self,
    device: &gpu::Device,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> Rc<gpu::Sampler> {
    let mut map = self.cache.borrow_mut();
    let desc = desc.into();
    map
      .entry(desc.clone()) // todo optimize move
      .or_insert_with(|| Rc::new(device.create_sampler(&desc.clone().into())))
      .clone()
  }
}

#[derive(Default)]
pub struct RenderPipelineCache {
  pub cache: RefCell<HashMap<u64, GPURenderPipeline>>,
}

pub trait ShaderHashProvider {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
}

pub trait ShaderHashProviderAny: ShaderHashProvider {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher);
}

impl<T> ShaderHashProviderAny for T
where
  T: ShaderHashProvider + Any,
{
  default fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.type_id().hash(hasher);
    self.hash_pipeline(hasher);
  }
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

impl RenderPipelineCache {
  pub fn get_or_insert_with(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce() -> GPURenderPipeline,
  ) -> GPURenderPipeline {
    let key = hasher.finish();
    self
      .cache
      .borrow_mut()
      .entry(key)
      .or_insert_with(creator)
      .clone()
  }
}
