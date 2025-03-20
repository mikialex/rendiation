use crate::*;

#[derive(Clone)]
pub struct GPUDevice {
  pub(crate) inner: Arc<GPUDeviceImpl>,
}
impl AsRef<Self> for GPUDevice {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl GPUDevice {
  pub(crate) fn new(device: gpu::Device) -> Self {
    let placeholder_bg = device.create_bind_group(&gpu::BindGroupDescriptor {
      layout: &device.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
        label: "PlaceholderBindgroup".into(),
        entries: &[],
      }),
      entries: &[],
      label: None,
    });

    let inner = GPUDeviceImpl {
      device,
      sampler_cache: Default::default(),
      bindgroup_cache: BindGroupCache::new(),
      bindgroup_layout_cache: Default::default(),
      render_pipeline_cache: Default::default(),
      compute_pipeline_cache: Default::default(),
      placeholder_bg: Arc::new(placeholder_bg),
      deferred_explicit_destroy: Default::default(),
      enable_binding_ty_check: Arc::new(RwLock::new(false)),
    };

    Self {
      inner: Arc::new(inner),
    }
  }

  pub fn set_binding_ty_check_enabled(&self, v: bool) {
    *self.inner.enable_binding_ty_check.write() = v;
  }

  pub fn get_binding_ty_check_enabled(&self) -> bool {
    *self.inner.enable_binding_ty_check.read()
  }

  pub fn create_cache_report(&self) -> GPUResourceCacheSizeReport {
    GPUResourceCacheSizeReport {
      bindgroup_count: self.inner.bindgroup_cache.cache.read().len(),
      bindgroup_layout_count: self.inner.bindgroup_layout_cache.cache.read().len(),
      sampler_count: self.inner.sampler_cache.cache.read().len(),
      pipeline_count: self.inner.render_pipeline_cache.read().len(),
    }
  }

  pub fn clear_resource_cache(&self) {
    self.inner.bindgroup_cache.clear();
    self.inner.bindgroup_layout_cache.clear();
    self.inner.sampler_cache.clear();
    let mut cache = self.inner.render_pipeline_cache.write();
    *cache = Default::default();
    let mut cache = self.inner.compute_pipeline_cache.write();
    *cache = Default::default();
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
    let mut cache = self.inner.render_pipeline_cache.write();

    let key = hasher.finish();
    cache.entry(key).or_insert_with(|| creator(self)).clone()
  }

  pub fn get_or_cache_create_compute_pipeline_by(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(&Self) -> GPUComputePipeline,
  ) -> GPUComputePipeline {
    let mut cache = self.inner.compute_pipeline_cache.write();

    let key = hasher.finish();
    cache.entry(key).or_insert_with(|| creator(self)).clone()
  }

  pub fn get_or_cache_create_compute_pipeline(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(ShaderComputePipelineBuilder) -> ShaderComputePipelineBuilder,
  ) -> GPUComputePipeline {
    self.get_or_cache_create_compute_pipeline_by(hasher, |device| {
      let builder = compute_shader_builder();
      let builder = creator(builder);
      builder.create_compute_pipeline(device).unwrap()
    })
  }

  pub fn create_and_cache_bindgroup_layout<'a>(
    &self,
    iter: impl IntoIterator<Item = (&'a ShaderBindingDescriptor, ShaderStages)> + Clone,
  ) -> GPUBindGroupLayout {
    let raw_layouts: Vec<_> = iter
      .into_iter()
      .enumerate()
      .map(|(i, (ty, vis))| map_shader_value_ty_to_binding_layout_type(ty, i, vis))
      .collect();

    let mut hasher = FastHasher::default();
    raw_layouts.hash(&mut hasher);
    let key = hasher.finish();

    self
      .inner
      .bindgroup_layout_cache
      .cache
      .write()
      .entry(key)
      .or_insert_with(|| {
        let inner = self.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
          label: None,
          entries: &raw_layouts,
        });
        GPUBindGroupLayout {
          inner,
          cache_id: key,
        }
      })
      .clone()
  }

  pub(crate) fn get_binding_cache(&self) -> &BindGroupCache {
    &self.inner.bindgroup_cache
  }

  pub fn make_indirect_dispatch_size_buffer(
    &self,
  ) -> StorageBufferDataView<DispatchIndirectArgsStorage> {
    let init = DispatchIndirectArgsStorage::default();

    let usage = gpu::BufferUsages::INDIRECT
      | gpu::BufferUsages::STORAGE
      | gpu::BufferUsages::COPY_DST
      | gpu::BufferUsages::COPY_SRC;

    let gpu = create_gpu_buffer(bytes_of(&init), usage, self).create_default_view();

    StorageBufferDataView {
      gpu,
      phantom: PhantomData,
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, Debug, Default, ShaderStruct)]
pub struct DispatchIndirectArgsStorage {
  /// The number of work groups in X dimension.
  pub x: u32,
  /// The number of work groups in Y dimension.
  pub y: u32,
  /// The number of work groups in Z dimension.
  pub z: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct GPUResourceCacheSizeReport {
  pub bindgroup_count: usize,
  pub bindgroup_layout_count: usize,
  pub sampler_count: usize,
  pub pipeline_count: usize,
}

pub(crate) struct GPUDeviceImpl {
  device: gpu::Device,
  sampler_cache: SamplerCache,
  bindgroup_cache: BindGroupCache,
  bindgroup_layout_cache: BindGroupLayoutCache,
  render_pipeline_cache: RwLock<FastHashMap<u64, GPURenderPipeline>>,
  compute_pipeline_cache: RwLock<FastHashMap<u64, GPUComputePipeline>>,
  pub(crate) deferred_explicit_destroy: DeferExplicitDestroy,
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
  pub(crate) enable_binding_ty_check: Arc<RwLock<bool>>,
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
    let mut map = self.cache.write();
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
    self.cache.write().clear();
  }
}

/// Note, here we not merge this trait into the graphics pipeline build trait because the hashing is
/// mainly used in pipeline caching, and the caching is optional by design.
///
/// Another strong version of the similar idea is: This approach could avoid accidental missing
/// hashing
///
/// ```
/// pub trait GraphicsPipelineVariant: GraphicsShaderProvider + Hash + Eq {}
/// pub trait GraphicsPipelineVariantProvider {
///   fn create_pipeline_variant(&self) -> Box<dyn GraphicsPipelineVariant>;
/// }
/// ```
/// however, we choose not use this approach for now because it will create much more heap
/// allocation
pub trait ShaderHashProvider {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}

  // if the type contains dynamic ShaderHashProvider, it's dynamic type info should be expressed in
  // hash pipeline, not in this fn
  fn hash_type_info(&self, hasher: &mut PipelineHasher);

  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_type_info(hasher);
    self.hash_pipeline(hasher);
  }
}

// pub trait ShaderVariantKeyProvider {
//   type VariantKey: Eq + Hash + Any;
//   fn create_variant_key(&self) -> Self::VariantKey;
// }
// impl<T: ShaderVariantKeyProvider> ShaderHashProvider for T {
//   fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
//     self.create_variant_key().hash(hasher);
//   }
//   fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
//     TypeId::of::<T::VariantKey>().hash(hasher)
//   }
// }

#[macro_export]
macro_rules! shader_hash_type_id {
  () => {
    fn hash_type_info(&self, hasher: &mut PipelineHasher) {
      use std::hash::Hash;
      std::any::TypeId::of::<Self>().hash(hasher);
    }
  };
  {$ty:ty} => {
    fn hash_type_info(&self, hasher: &mut PipelineHasher) {
      use std::hash::Hash;
      std::any::TypeId::of::<$ty>().hash(hasher);
    }
  };
}

impl ShaderHashProvider for () {
  shader_hash_type_id! {}
}

/// User could use this to debug if the hashing logic issue
pub struct DebugHasher<T> {
  hash_history: Vec<(Vec<u8>, std::backtrace::Backtrace)>,
  hasher: T,
}

impl<T> DebugHasher<T> {
  pub fn debug_print_previous_hash_stacks(&self) {
    println!("{:#?}", self.hash_history);
  }
}

impl<T: std::hash::Hasher> std::hash::Hasher for DebugHasher<T> {
  fn finish(&self) -> u64 {
    self.hasher.finish()
  }

  fn write(&mut self, bytes: &[u8]) {
    self
      .hash_history
      .push((Vec::from(bytes), std::backtrace::Backtrace::force_capture()));
    self.hasher.write(bytes)
  }
}

#[derive(Default)]
pub struct PipelineHasher<T = FastHasher> {
  hasher: T,
}

impl<T> PipelineHasher<T> {
  pub fn into_debugger(self) -> DebugHasher<Self> {
    DebugHasher {
      hasher: self,
      hash_history: Default::default(),
    }
  }

  pub fn with_hash(mut self, h: impl Hash) -> Self
  where
    Self: Hasher,
  {
    h.hash(&mut self);
    self
  }
}

impl std::hash::Hasher for PipelineHasher {
  fn finish(&self) -> u64 {
    self.hasher.finish()
  }

  fn write(&mut self, bytes: &[u8]) {
    self.hasher.write(bytes)
  }
}

#[macro_export]
macro_rules! shader_hasher_from_marker_ty {
  ($ty: tt) => {{
    struct $ty;
    PipelineHasher::default().with_hash(std::any::TypeId::of::<$ty>())
  }};
}
