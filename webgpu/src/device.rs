use crate::*;

#[derive(Clone)]
pub struct GPUDevice {
  inner: Rc<GPUDeviceInner>,
}

impl GPUDevice {
  pub fn new(device: wgpu::Device) -> Self {
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

  pub fn create_and_cache_render_pipeline(
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
        let inner = self.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
  device: wgpu::Device,
  sampler_cache: SamplerCache,
  bindgroup_cache: BindGroupCache,
  bindgroup_layout_cache: BindGroupLayoutCache,
  pipeline_cache: RenderPipelineCache,
}

impl Deref for GPUDevice {
  type Target = wgpu::Device;

  fn deref(&self) -> &Self::Target {
    &self.inner.device
  }
}

pub type RawSampler = Rc<wgpu::Sampler>;
#[derive(Default)]
pub struct SamplerCache {
  cache: RefCell<HashMap<GPUSamplerDescriptor, RawSampler>>,
}

impl SamplerCache {
  pub fn retrieve(
    &self,
    device: &wgpu::Device,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> Rc<wgpu::Sampler> {
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
