use wgpu::{custom::*, Device, Extent3d};

#[derive(Debug)]
pub struct InstrumentedDevice {
  internal: DispatchDevice,
  stat: Arc<RwLock<DeviceStatistics>>,
}

impl InstrumentedDevice {
  pub fn wrap(internal: Device) -> (Device, Arc<RwLock<DeviceStatistics>>) {
    let buffer_stat: Arc<RwLock<DeviceStatistics>> = Default::default();
    let device = Device::from_custom(InstrumentedDevice {
      internal: internal.into(),
      stat: buffer_stat.clone(),
    });
    (device, buffer_stat)
  }
}

#[derive(Debug, Default)]
pub struct DeviceStatistics {
  pub buffer_instance_count: u64,
  /// this byte size is only an estimation
  pub buffer_byte_count: u64,
  pub texture_instance_count: u64,
  /// this byte size is only an estimation
  pub texture_byte_count: u64,
  pub texture_view_instance_count: u64,
  pub bind_group_instance_count: u64,
}

#[derive(Debug)]
struct InstrumentedBuffer {
  byte_size: u64,
  internal: DispatchBuffer,
  stat: Arc<RwLock<DeviceStatistics>>,
}

impl BufferInterface for InstrumentedBuffer {
  fn map_async(
    &self,
    mode: wgpu::MapMode,
    range: std::ops::Range<wgpu::BufferAddress>,
    callback: BufferMapCallback,
  ) {
    self.internal.map_async(mode, range, callback);
  }

  fn get_mapped_range(
    &self,
    sub_range: std::ops::Range<wgpu::BufferAddress>,
  ) -> DispatchBufferMappedRange {
    self.internal.get_mapped_range(sub_range)
  }

  fn unmap(&self) {
    self.internal.unmap();
  }

  fn destroy(&self) {
    self.internal.destroy();
    let mut stat = self.stat.write().unwrap();
    stat.buffer_byte_count -= self.byte_size;
    stat.buffer_instance_count -= 1;
  }
}

#[derive(Debug)]
struct InstrumentedTexture {
  byte_size: u64,
  internal: DispatchTexture,
  stat: Arc<RwLock<DeviceStatistics>>,
}

impl TextureInterface for InstrumentedTexture {
  fn destroy(&self) {
    self.internal.destroy();
    let mut stat = self.stat.write().unwrap();
    stat.texture_instance_count -= 1;
    stat.texture_byte_count -= self.byte_size;
  }

  fn create_view(&self, desc: &wgpu::TextureViewDescriptor<'_>) -> DispatchTextureView {
    self.stat.write().unwrap().texture_view_instance_count += 1;
    DispatchTextureView::custom(InstrumentedTextureView {
      _internal: self.internal.create_view(desc),
      stat: self.stat.clone(),
    })
  }
}

#[derive(Debug)]
struct InstrumentedTextureView {
  _internal: DispatchTextureView,
  stat: Arc<RwLock<DeviceStatistics>>,
}

// TextureViewInterface does not has destroy method
impl Drop for InstrumentedTextureView {
  fn drop(&mut self) {
    let mut stat = self.stat.write().unwrap();
    stat.texture_view_instance_count -= 1;
  }
}

impl TextureViewInterface for InstrumentedTextureView {}

#[derive(Debug)]
struct InstrumentedBindGroup {
  _internal: DispatchBindGroup,
  stat: Arc<RwLock<DeviceStatistics>>,
}

impl BindGroupInterface for InstrumentedBindGroup {}
// BindGroupInterface does not has destroy method
impl Drop for InstrumentedBindGroup {
  fn drop(&mut self) {
    let mut stat = self.stat.write().unwrap();
    stat.bind_group_instance_count -= 1;
  }
}

impl DeviceInterface for InstrumentedDevice {
  fn features(&self) -> wgpu::Features {
    self.internal.features()
  }

  fn limits(&self) -> wgpu::Limits {
    self.internal.limits()
  }

  fn create_shader_module(
    &self,
    desc: wgpu::ShaderModuleDescriptor<'_>,
    shader_bound_checks: wgpu::ShaderRuntimeChecks,
  ) -> wgpu::custom::DispatchShaderModule {
    self
      .internal
      .create_shader_module(desc, shader_bound_checks)
  }

  unsafe fn create_shader_module_passthrough(
    &self,
    desc: &wgpu::ShaderModuleDescriptorPassthrough<'_>,
  ) -> wgpu::custom::DispatchShaderModule {
    self.internal.create_shader_module_passthrough(desc)
  }

  fn create_bind_group_layout(
    &self,
    desc: &wgpu::BindGroupLayoutDescriptor<'_>,
  ) -> wgpu::custom::DispatchBindGroupLayout {
    self.internal.create_bind_group_layout(desc)
  }

  fn create_bind_group(
    &self,
    desc: &wgpu::BindGroupDescriptor<'_>,
  ) -> wgpu::custom::DispatchBindGroup {
    self.stat.write().unwrap().bind_group_instance_count += 1;
    wgpu::custom::DispatchBindGroup::custom(InstrumentedBindGroup {
      _internal: self.internal.create_bind_group(desc),
      stat: self.stat.clone(),
    })
  }

  fn create_pipeline_layout(
    &self,
    desc: &wgpu::PipelineLayoutDescriptor<'_>,
  ) -> wgpu::custom::DispatchPipelineLayout {
    self.internal.create_pipeline_layout(desc)
  }

  fn create_render_pipeline(
    &self,
    desc: &wgpu::RenderPipelineDescriptor<'_>,
  ) -> wgpu::custom::DispatchRenderPipeline {
    self.internal.create_render_pipeline(desc)
  }

  fn create_compute_pipeline(
    &self,
    desc: &wgpu::ComputePipelineDescriptor<'_>,
  ) -> wgpu::custom::DispatchComputePipeline {
    self.internal.create_compute_pipeline(desc)
  }

  unsafe fn create_pipeline_cache(
    &self,
    desc: &wgpu::PipelineCacheDescriptor<'_>,
  ) -> wgpu::custom::DispatchPipelineCache {
    self.internal.create_pipeline_cache(desc)
  }

  fn create_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> wgpu::custom::DispatchBuffer {
    let buffer = self.internal.create_buffer(desc);
    let stat = self.stat.clone();
    let byte_size = desc.size;
    {
      let mut stat = stat.write().unwrap();
      stat.buffer_instance_count += 1;
      stat.buffer_byte_count += byte_size;
    }

    wgpu::custom::DispatchBuffer::custom(InstrumentedBuffer {
      internal: buffer,
      stat,
      byte_size,
    })
  }

  fn create_texture(&self, desc: &wgpu::TextureDescriptor<'_>) -> wgpu::custom::DispatchTexture {
    let texture = self.internal.create_texture(desc);
    let stat = self.stat.clone();

    let mut level_size = desc.size;
    let mut byte_size = 0;
    for _ in 0..desc.mip_level_count {
      byte_size += desc.format.theoretical_memory_footprint(level_size);
      level_size = Extent3d {
        width: level_size.width >> 1,
        height: level_size.height >> 1,
        depth_or_array_layers: level_size.depth_or_array_layers >> 1,
      }
    }
    {
      let mut stat = stat.write().unwrap();
      stat.texture_instance_count += 1;
      stat.texture_byte_count += byte_size;
    }
    wgpu::custom::DispatchTexture::custom(InstrumentedTexture {
      byte_size,
      internal: texture,
      stat,
    })
  }

  fn create_blas(
    &self,
    desc: &wgpu::CreateBlasDescriptor<'_>,
    sizes: wgpu::BlasGeometrySizeDescriptors,
  ) -> (Option<u64>, wgpu::custom::DispatchBlas) {
    self.internal.create_blas(desc, sizes)
  }

  fn create_tlas(&self, desc: &wgpu::CreateTlasDescriptor<'_>) -> wgpu::custom::DispatchTlas {
    self.internal.create_tlas(desc)
  }

  fn create_sampler(&self, desc: &wgpu::SamplerDescriptor<'_>) -> wgpu::custom::DispatchSampler {
    self.internal.create_sampler(desc)
  }

  fn create_query_set(
    &self,
    desc: &wgpu::QuerySetDescriptor<'_>,
  ) -> wgpu::custom::DispatchQuerySet {
    self.internal.create_query_set(desc)
  }

  fn create_command_encoder(
    &self,
    desc: &wgpu::CommandEncoderDescriptor<'_>,
  ) -> wgpu::custom::DispatchCommandEncoder {
    self.internal.create_command_encoder(desc)
  }

  fn create_render_bundle_encoder(
    &self,
    desc: &wgpu::RenderBundleEncoderDescriptor<'_>,
  ) -> wgpu::custom::DispatchRenderBundleEncoder {
    self.internal.create_render_bundle_encoder(desc)
  }

  fn set_device_lost_callback(&self, device_lost_callback: wgpu::custom::BoxDeviceLostCallback) {
    self.internal.set_device_lost_callback(device_lost_callback)
  }

  fn on_uncaptured_error(&self, handler: Box<dyn wgpu::UncapturedErrorHandler>) {
    self.internal.on_uncaptured_error(handler)
  }

  fn push_error_scope(&self, filter: wgpu::ErrorFilter) {
    self.internal.push_error_scope(filter)
  }

  fn pop_error_scope(&self) -> std::pin::Pin<Box<dyn wgpu::custom::PopErrorScopeFuture>> {
    self.internal.pop_error_scope()
  }

  unsafe fn start_graphics_debugger_capture(&self) {
    self.internal.start_graphics_debugger_capture()
  }

  unsafe fn stop_graphics_debugger_capture(&self) {
    self.internal.stop_graphics_debugger_capture()
  }

  fn poll(&self, poll_type: wgpu::wgt::PollType<u64>) -> Result<wgpu::PollStatus, wgpu::PollError> {
    self.internal.poll(poll_type)
  }

  fn get_internal_counters(&self) -> wgpu::InternalCounters {
    self.internal.get_internal_counters()
  }

  fn generate_allocator_report(&self) -> Option<wgpu::AllocatorReport> {
    self.internal.generate_allocator_report()
  }

  fn destroy(&self) {
    self.internal.destroy();
  }
}
