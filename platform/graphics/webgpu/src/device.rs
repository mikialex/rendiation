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
  pub(crate) fn new(
    device: gpu::Device,
    info: GPUInfo,
    default_shader_checks: ShaderRuntimeChecks,
  ) -> Self {
    let counters = Arc::new(GPUCallCounters::default());
    counters.device.create_bind_group_layout.inc();
    counters.device.create_bind_group.inc();
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
      deferred_explicit_destroy: DeferExplicitDestroy::new(counters.clone()),
      counters,
      sampler_cache: Default::default(),
      bindgroup_cache: Default::default(),
      bindgroup_layout_cache: Default::default(),
      render_pipeline_cache: Default::default(),
      compute_pipeline_cache: Default::default(),
      placeholder_bg,
      enable_binding_ty_check: AtomicBool::new(cfg!(debug_assertions)),
      default_shader_checks,
      info,
    };

    Self {
      inner: Arc::new(inner),
    }
  }

  pub fn info(&self) -> &GPUInfo {
    &self.inner.info
  }

  pub fn set_binding_ty_check_enabled(&self, v: bool) {
    self
      .inner
      .enable_binding_ty_check
      .store(v, Ordering::Relaxed);
  }

  pub fn get_binding_ty_check_enabled(&self) -> bool {
    self.inner.enable_binding_ty_check.load(Ordering::Relaxed)
  }

  pub fn create_cache_report(&self) -> GPUResourceCacheSizeReport {
    GPUResourceCacheSizeReport {
      bindgroup_count: self
        .inner
        .bindgroup_cache
        .cache
        .read()
        .cached_binding_count(),
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
    self.create_encoder_with_label(None)
  }

  pub fn create_encoder_with_label(&self, label: Option<&str>) -> GPUCommandEncoder {
    self.inner.counters.device.create_command_encoder.inc();
    let encoder = self
      .inner
      .device
      .create_command_encoder(&gpu::CommandEncoderDescriptor { label });
    GPUCommandEncoder::new(encoder, self)
  }

  pub fn create_and_cache_sampler(&self, desc: impl Into<GPUSamplerDescriptor>) -> RawSampler {
    self.create_and_cache_sampler_view(desc).view.clone()
  }

  /// the returned view is shared for the same sampler descriptor, so the view id is stable and the
  /// bindgroup cache can be hit. prefer this over creating new sampler view each time.
  pub fn create_and_cache_sampler_view(
    &self,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> GPUSamplerView {
    self.inner.sampler_cache.retrieve(self, desc)
  }

  pub fn get_or_cache_create_render_pipeline(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(&Self, &str) -> GPURenderPipeline,
  ) -> GPURenderPipeline {
    let mut cache = self.inner.render_pipeline_cache.write();

    #[cfg(feature = "pipeline-label")]
    {
      let (key, label) = hasher.finish_with_label();
      cache
        .entry(key)
        .or_insert_with(|| creator(self, &label))
        .clone()
    }
    #[cfg(not(feature = "pipeline-label"))]
    {
      let key = hasher.finish();
      cache
        .entry(key)
        .or_insert_with(|| creator(self, ""))
        .clone()
    }
  }

  pub fn get_or_cache_create_compute_pipeline(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(&Self, &str) -> GPUComputePipeline,
  ) -> GPUComputePipeline {
    let mut cache = self.inner.compute_pipeline_cache.write();

    #[cfg(feature = "pipeline-label")]
    {
      let (key, label) = hasher.finish_with_label();
      cache
        .entry(key)
        .or_insert_with(|| creator(self, &label))
        .clone()
    }
    #[cfg(not(feature = "pipeline-label"))]
    {
      let key = hasher.finish();
      cache
        .entry(key)
        .or_insert_with(|| creator(self, ""))
        .clone()
    }
  }

  pub fn get_or_cache_create_compute_pipeline_by(
    &self,
    hasher: PipelineHasher,
    creator: impl FnOnce(ShaderComputePipelineBuilder) -> ShaderComputePipelineBuilder,
  ) -> GPUComputePipeline {
    self.get_or_cache_create_compute_pipeline(hasher, |device, label| {
      let builder = compute_shader_builder(self);
      let builder = creator(builder);
      builder.create_compute_pipeline(device, label).unwrap()
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

    self
      .inner
      .bindgroup_layout_cache
      .cache
      .write()
      .entry(raw_layouts)
      .or_insert_with_key(|raw_layouts| {
        let inner = self.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
          label: None,
          entries: raw_layouts,
        });
        GPUBindGroupLayout {
          inner,
          cache_id: new_bindgroup_layout_id(),
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
  pub(crate) counters: Arc<GPUCallCounters>,
  pub(crate) placeholder_bg: gpu::BindGroup,
  pub(crate) enable_binding_ty_check: AtomicBool,
  pub(crate) default_shader_checks: ShaderRuntimeChecks,
  /// this info is as same as the gpu, we clone here for easy access
  info: GPUInfo,
}

/// The wgpu device api wrapper, the calls are counted, see [GPUCallCountReport].
impl GPUDevice {
  /// Access the underlying wgpu device.
  ///
  /// The calls made through the raw device are not counted in [GPUCallCountReport]. Only use it
  /// when the wgpu type is required, for example to integrate the third party renderer. Any common
  /// usage should use or add the wrapper method in [GPUDevice].
  pub fn raw_device(&self) -> &gpu::Device {
    &self.inner.device
  }

  pub fn poll(&self, poll_type: gpu::PollType) -> Result<gpu::PollStatus, gpu::PollError> {
    self.inner.counters.device.device_poll.inc();
    self.inner.device.poll(poll_type)
  }

  pub fn features(&self) -> Features {
    self.inner.device.features()
  }

  pub fn limits(&self) -> Limits {
    self.inner.device.limits()
  }

  pub fn adapter_info(&self) -> gpu::AdapterInfo {
    self.inner.device.adapter_info()
  }

  pub fn create_shader_module(&self, desc: gpu::ShaderModuleDescriptor) -> gpu::ShaderModule {
    self.inner.counters.device.create_shader_module.inc();
    self.inner.device.create_shader_module(desc)
  }

  /// # Safety
  ///
  /// see [gpu::Device::create_shader_module_trusted]
  pub unsafe fn create_shader_module_trusted(
    &self,
    desc: gpu::ShaderModuleDescriptor,
    runtime_checks: ShaderRuntimeChecks,
  ) -> gpu::ShaderModule {
    self.inner.counters.device.create_shader_module.inc();
    unsafe {
      self
        .inner
        .device
        .create_shader_module_trusted(desc, runtime_checks)
    }
  }

  /// # Safety
  ///
  /// see [gpu::Device::create_shader_module_passthrough]
  pub unsafe fn create_shader_module_passthrough(
    &self,
    desc: gpu::ShaderModuleDescriptorPassthrough,
  ) -> gpu::ShaderModule {
    self.inner.counters.device.create_shader_module.inc();
    unsafe { self.inner.device.create_shader_module_passthrough(desc) }
  }

  /// note, the calls on the returned bundle encoder are not counted
  pub fn create_render_bundle_encoder<'a>(
    &self,
    desc: &gpu::RenderBundleEncoderDescriptor,
  ) -> gpu::RenderBundleEncoder<'a> {
    self
      .inner
      .counters
      .device
      .create_render_bundle_encoder
      .inc();
    self.inner.device.create_render_bundle_encoder(desc)
  }

  pub fn create_bind_group(&self, desc: &gpu::BindGroupDescriptor) -> gpu::BindGroup {
    self.inner.counters.device.create_bind_group.inc();
    self.inner.device.create_bind_group(desc)
  }

  pub fn create_bind_group_layout(
    &self,
    desc: &gpu::BindGroupLayoutDescriptor,
  ) -> gpu::BindGroupLayout {
    self.inner.counters.device.create_bind_group_layout.inc();
    self.inner.device.create_bind_group_layout(desc)
  }

  pub fn create_pipeline_layout(
    &self,
    desc: &gpu::PipelineLayoutDescriptor,
  ) -> gpu::PipelineLayout {
    self.inner.counters.device.create_pipeline_layout.inc();
    self.inner.device.create_pipeline_layout(desc)
  }

  pub fn create_render_pipeline(
    &self,
    desc: &gpu::RenderPipelineDescriptor,
  ) -> gpu::RenderPipeline {
    self.inner.counters.device.create_render_pipeline.inc();
    self.inner.device.create_render_pipeline(desc)
  }

  pub fn create_mesh_pipeline(&self, desc: &gpu::MeshPipelineDescriptor) -> gpu::RenderPipeline {
    self.inner.counters.device.create_mesh_pipeline.inc();
    self.inner.device.create_mesh_pipeline(desc)
  }

  pub fn create_compute_pipeline(
    &self,
    desc: &gpu::ComputePipelineDescriptor,
  ) -> gpu::ComputePipeline {
    self.inner.counters.device.create_compute_pipeline.inc();
    self.inner.device.create_compute_pipeline(desc)
  }

  /// # Safety
  ///
  /// see [gpu::Device::create_pipeline_cache]
  pub unsafe fn create_pipeline_cache(
    &self,
    desc: &gpu::PipelineCacheDescriptor,
  ) -> gpu::PipelineCache {
    self.inner.counters.device.create_pipeline_cache.inc();
    unsafe { self.inner.device.create_pipeline_cache(desc) }
  }

  pub fn create_buffer(&self, desc: &gpu::BufferDescriptor) -> gpu::Buffer {
    let counters = &self.inner.counters.device;
    counters.create_buffer.inc();
    counters.create_buffer_bytes.add(desc.size);
    self.inner.device.create_buffer(desc)
  }

  /// see [gpu::util::DeviceExt::create_buffer_init]
  pub fn create_buffer_init(&self, desc: &gpu::util::BufferInitDescriptor) -> gpu::Buffer {
    let counters = &self.inner.counters.device;
    counters.create_buffer.inc();
    // the size is padded to the copy alignment and at least one alignment, same as wgpu does
    let size = (desc.contents.len() as u64)
      .max(1)
      .next_multiple_of(gpu::COPY_BUFFER_ALIGNMENT);
    counters.create_buffer_bytes.add(size);
    counters
      .create_buffer_init_bytes
      .add(desc.contents.len() as u64);
    self.inner.device.create_buffer_init(desc)
  }

  pub fn create_texture(&self, desc: &gpu::TextureDescriptor) -> gpu::Texture {
    self.inner.counters.device.create_texture.inc();
    self.inner.device.create_texture(desc)
  }

  pub fn create_external_texture(
    &self,
    desc: &gpu::ExternalTextureDescriptor,
    planes: &[&gpu::TextureView],
  ) -> gpu::ExternalTexture {
    self.inner.counters.device.create_external_texture.inc();
    self.inner.device.create_external_texture(desc, planes)
  }

  pub fn create_sampler(&self, desc: &gpu::SamplerDescriptor) -> gpu::Sampler {
    self.inner.counters.device.create_sampler.inc();
    self.inner.device.create_sampler(desc)
  }

  pub fn create_query_set(&self, desc: &gpu::QuerySetDescriptor) -> gpu::QuerySet {
    self.inner.counters.device.create_query_set.inc();
    self.inner.device.create_query_set(desc)
  }

  pub fn create_blas(
    &self,
    desc: &gpu::CreateBlasDescriptor,
    sizes: gpu::BlasGeometrySizeDescriptors,
  ) -> gpu::Blas {
    self.inner.counters.device.create_blas.inc();
    self.inner.device.create_blas(desc, sizes)
  }

  pub fn create_tlas(&self, desc: &gpu::CreateTlasDescriptor) -> gpu::Tlas {
    self.inner.counters.device.create_tlas.inc();
    self.inner.device.create_tlas(desc)
  }

  pub fn on_uncaptured_error(&self, handler: Arc<dyn gpu::UncapturedErrorHandler>) {
    self.inner.device.on_uncaptured_error(handler)
  }

  pub fn push_error_scope(&self, filter: gpu::ErrorFilter) -> gpu::ErrorScopeGuard {
    self.inner.device.push_error_scope(filter)
  }

  pub fn set_device_lost_callback(
    &self,
    callback: impl Fn(gpu::DeviceLostReason, String) + Send + 'static,
  ) {
    self.inner.device.set_device_lost_callback(callback)
  }

  /// # Safety
  ///
  /// see [gpu::Device::start_graphics_debugger_capture]
  pub unsafe fn start_graphics_debugger_capture(&self) {
    unsafe { self.inner.device.start_graphics_debugger_capture() }
  }

  /// # Safety
  ///
  /// see [gpu::Device::stop_graphics_debugger_capture]
  pub unsafe fn stop_graphics_debugger_capture(&self) {
    unsafe { self.inner.device.stop_graphics_debugger_capture() }
  }

  pub fn get_internal_counters(&self) -> wgpu_types::InternalCounters {
    self.inner.device.get_internal_counters()
  }

  pub fn generate_allocator_report(&self) -> Option<wgpu_types::AllocatorReport> {
    self.inner.device.generate_allocator_report()
  }

  pub fn destroy(&self) {
    self.inner.device.destroy()
  }
}

#[derive(Default)]
pub struct SamplerCache {
  cache: RwLock<FastHashMap<GPUSamplerDescriptor, GPUSamplerView>>,
}

impl SamplerCache {
  pub fn retrieve(
    &self,
    device: &GPUDevice,
    desc: impl Into<GPUSamplerDescriptor>,
  ) -> GPUSamplerView {
    let desc = desc.into();
    if let Some(view) = self.cache.read().get(&desc) {
      return view.clone();
    }

    let mut map = self.cache.write();
    map
      .raw_entry_mut()
      .from_key(&desc)
      .or_insert_with(|| {
        let raw_desc: gpu::SamplerDescriptor<'static> = desc.clone().into();
        let raw = RawSampler(device.create_sampler(&raw_desc));
        let view = GPUSampler::create_with_raw(raw, raw_desc, device).create_default_view();
        (desc.clone(), view)
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
/// ```ignore
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

#[macro_export]
macro_rules! shader_hash_type_id {
  () => {
    fn hash_type_info(&self, hasher: &mut PipelineHasher) {
      hasher.hash_type::<Self>();
    }
  };
  {$ty:ty} => {
    fn hash_type_info(&self, hasher: &mut PipelineHasher) {
      hasher.hash_type::<$ty>();
    }
  };
}

impl ShaderHashProvider for () {
  shader_hash_type_id! {}
}
impl<T: ShaderHashProvider> ShaderHashProvider for &T {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    (*self).hash_type_info(hasher);
  }

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline(hasher);
  }
}

#[derive(Default)]
pub struct PipelineHasher<T = FastHasher> {
  hasher: T,
  #[cfg(feature = "pipeline-label")]
  debug_label: String,
  // we could further do this:
  // hash_history: Vec<std::backtrace::Backtrace>,
}

impl<T: Hasher> PipelineHasher<T> {
  #[inline(always)]
  pub fn with_hash(mut self, h: impl Hash + Debug) -> Self {
    self.hash(h);
    self
  }

  #[inline(always)]
  pub fn with_hash_type<X: 'static>(mut self) -> Self {
    self.hash_type::<X>();
    self
  }

  #[allow(unused)]
  pub fn label_name(&mut self, name: &str) -> &mut Self {
    #[cfg(feature = "pipeline-label")]
    {
      self.debug_label = format!("{} {}", name, self.debug_label);
    }
    self
  }

  pub fn hash(&mut self, h: impl Hash + Debug) -> &mut Self {
    h.hash(&mut self.hasher);
    #[cfg(feature = "pipeline-label")]
    {
      if !self.debug_label.is_empty() {
        self.debug_label.push('+');
      }
      self.debug_label.push_str(&format!("{:?}", h));
    }

    self
  }

  pub fn hash_type<X: 'static>(&mut self) -> &mut Self {
    std::any::TypeId::of::<X>().hash(&mut self.hasher);
    #[cfg(feature = "pipeline-label")]
    {
      if !self.debug_label.is_empty() {
        self.debug_label.push('-');
      }
      let name = disqualified::ShortName(std::any::type_name::<X>());
      self.debug_label.push_str(&format!("{:?}", name));
    }

    self
  }

  pub fn finish(self) -> u64 {
    self.hasher.finish()
  }
  pub fn finish_with_label(self) -> (u64, String) {
    #[cfg(feature = "pipeline-label")]
    {
      (self.hasher.finish(), self.debug_label)
    }

    #[cfg(not(feature = "pipeline-label"))]
    {
      (self.hasher.finish(), String::default())
    }
  }
}

#[macro_export]
macro_rules! shader_hasher_from_marker_ty {
  ($ty: tt) => {{
    struct $ty;
    PipelineHasher::default().with_hash_type::<$ty>()
  }};
}
