use crate::*;

#[derive(Clone)]
pub struct GPUDevice {
  pub(crate) inner: Rc<GPUDeviceInner>,
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
      bindgroup_cache: Default::default(),
      bindgroup_layout_cache: Default::default(),
      pipeline_cache: Default::default(),
      placeholder_bg: Rc::new(placeholder_bg),
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

  pub(crate) fn create_and_cache_com_sampler(
    &self,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> RawComparisonSampler {
    self
      .inner
      .sampler_cache
      .retrieve_comparison(&self.inner.device, desc)
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

pub(crate) struct GPUDeviceInner {
  device: gpu::Device,
  sampler_cache: SamplerCache,
  bindgroup_cache: BindGroupCache,
  bindgroup_layout_cache: BindGroupLayoutCache,
  pipeline_cache: RenderPipelineCache,
  pub(crate) placeholder_bg: Rc<gpu::BindGroup>,
}

impl Deref for GPUDevice {
  type Target = gpu::Device;

  fn deref(&self) -> &Self::Target {
    &self.inner.device
  }
}

#[derive(Clone)]
pub struct RawSampler(pub Rc<gpu::Sampler>);

#[derive(Clone)]
pub struct RawComparisonSampler(pub Rc<gpu::Sampler>);

#[derive(Default)]
pub struct SamplerCache {
  cache: RefCell<HashMap<GPUSamplerDescriptor, RawSampler>>,
  cache_compare: RefCell<HashMap<GPUSamplerDescriptor, RawComparisonSampler>>,
}

impl SamplerCache {
  pub fn retrieve(
    &self,
    device: &gpu::Device,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> RawSampler {
    let mut map = self.cache.borrow_mut();
    let desc = desc.into();
    map
      .entry(desc.clone()) // todo optimize move
      .or_insert_with(|| RawSampler(Rc::new(device.create_sampler(&desc.clone().into()))))
      .clone()
  }

  pub fn retrieve_comparison(
    &self,
    device: &gpu::Device,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> RawComparisonSampler {
    let mut map = self.cache_compare.borrow_mut();
    let mut desc = desc.into();
    desc.compare = None;
    map
      .entry(desc.clone()) // todo optimize move
      .or_insert_with(|| RawComparisonSampler(Rc::new(device.create_sampler(&desc.clone().into()))))
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
