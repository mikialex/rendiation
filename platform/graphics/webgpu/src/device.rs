use crate::*;

#[derive(Clone)]
pub struct GPUDevice {
  pub(crate) inner: Arc<GPUDeviceInner>,
}
impl AsRef<Self> for GPUDevice {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl GPUDevice {
  pub fn new(device: gpu::Device) -> Self {
    let placeholder_bg = device.create_bind_group(&gpu::BindGroupDescriptor {
      layout: &device.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
        label: "PlaceholderBindgroup".into(),
        entries: &[],
      }),
      entries: &[],
      label: None,
    });

    let inner = GPUDeviceInner {
      device,
      sampler_cache: Default::default(),
      bindgroup_cache: BindGroupCache::new(),
      bindgroup_layout_cache: Default::default(),
      pipeline_cache: Default::default(),
      placeholder_bg: Arc::new(placeholder_bg),
    };

    Self {
      inner: Arc::new(inner),
    }
  }

  pub fn create_cache_report(&self) -> GPUResourceCacheSizeReport {
    GPUResourceCacheSizeReport {
      bindgroup_count: self.inner.bindgroup_cache.cache.read().unwrap().len(),
      bindgroup_layout_count: self
        .inner
        .bindgroup_layout_cache
        .cache
        .read()
        .unwrap()
        .len(),
      sampler_count: self.inner.sampler_cache.cache.read().unwrap().len(),
      pipeline_count: self.inner.pipeline_cache.cache.read().unwrap().len(),
    }
  }

  pub fn clear_resource_cache(&self) {
    self.inner.bindgroup_cache.clear();
    self.inner.bindgroup_layout_cache.clear();
    self.inner.sampler_cache.clear();
    self.inner.pipeline_cache.clear();
  }

  pub fn create_encoder(&self) -> GPUCommandEncoder {
    let encoder = self.create_command_encoder(&gpu::CommandEncoderDescriptor { label: None });
    GPUCommandEncoder::new(encoder, self)
  }

  pub fn create_and_cache_sampler(&self, desc: impl Into<GPUSamplerDescriptor>) -> RawSampler {
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
    layouts: &[gpu::BindGroupLayoutEntry],
  ) -> GPUBindGroupLayout {
    let mut hasher = FastHasher::default();
    layouts.hash(&mut hasher);
    let key = hasher.finish();

    self
      .inner
      .bindgroup_layout_cache
      .cache
      .write()
      .unwrap()
      .entry(key)
      .or_insert_with(|| {
        let inner = self.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
          label: None,
          entries: layouts,
        });
        GPUBindGroupLayout {
          inner: Arc::new(inner),
          cache_id: key,
        }
      })
      .clone()
  }

  pub(crate) fn get_binding_cache(&self) -> &BindGroupCache {
    &self.inner.bindgroup_cache
  }
}

#[derive(Debug, Copy, Clone)]
pub struct GPUResourceCacheSizeReport {
  pub bindgroup_count: usize,
  pub bindgroup_layout_count: usize,
  pub sampler_count: usize,
  pub pipeline_count: usize,
}

pub(crate) struct GPUDeviceInner {
  device: gpu::Device,
  sampler_cache: SamplerCache,
  bindgroup_cache: BindGroupCache,
  bindgroup_layout_cache: BindGroupLayoutCache,
  pipeline_cache: RenderPipelineCache,
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
}

impl Deref for GPUDevice {
  type Target = gpu::Device;

  fn deref(&self) -> &Self::Target {
    &self.inner.device
  }
}

#[derive(Default)]
pub struct SamplerCache {
  cache: RwLock<FastHashMap<GPUSamplerDescriptor, RawSampler>>,
}

impl SamplerCache {
  pub fn retrieve(
    &self,
    device: &gpu::Device,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> RawSampler {
    let mut map = self.cache.write().unwrap();
    let desc = desc.into();
    map
      .raw_entry_mut()
      .from_key(&desc)
      .or_insert_with(|| {
        (
          desc.clone(),
          RawSampler(Arc::new(device.create_sampler(&desc.clone().into()))),
        )
      })
      .1
      .clone()
  }
  pub(crate) fn clear(&self) {
    self.cache.write().unwrap().clear();
  }
}

#[derive(Default)]
pub struct RenderPipelineCache {
  pub cache: RwLock<FastHashMap<u64, GPURenderPipeline>>,
}

pub trait ShaderHashProvider {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
}

impl ShaderHashProvider for () {}

/// Some type is not 'static, which is not impl Any, but we still require shader hash
/// impl with itself's type identity info. In this case, the user should impl this trait
/// manually.
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
  hasher: FastHasher,
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
      .write()
      .unwrap()
      .entry(key)
      .or_insert_with(creator)
      .clone()
  }
  pub(crate) fn clear(&self) {
    self.cache.write().unwrap().clear();
  }
}
