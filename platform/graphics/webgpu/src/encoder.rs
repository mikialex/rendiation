use crate::*;

pub struct GPUCommandEncoder {
  encoder: gpu::CommandEncoder,
  placeholder_bg: gpu::BindGroup,
  deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
  pub(crate) on_submit: EventSource<()>,
  counts: GPUCommandCallCounts,
  pass_counts: PassCallCountSink,
}

pub struct GPUCommandBuffer {
  pub(crate) inner: gpu::CommandBuffer,
  pub(crate) on_submit: EventSource<()>,
  pub(crate) counts: GPUCommandCallCounts,
  pub(crate) _deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
}

impl GPUCommandBuffer {
  /// the recorded call counts of this command buffer
  pub fn call_counts(&self) -> &GPUCommandCallCounts {
    &self.counts
  }
}

impl GPUCommandEncoder {
  pub(crate) fn new(encoder: gpu::CommandEncoder, device: &GPUDevice) -> Self {
    Self {
      encoder,
      placeholder_bg: device.inner.placeholder_bg.clone(),
      on_submit: Default::default(),
      deferred_explicit_destroy: device.inner.deferred_explicit_destroy.new_command_buffer(),
      counts: Default::default(),
      pass_counts: Default::default(),
    }
  }

  /// the recorded call counts so far, the counts of the passes are included after the pass dropped
  pub fn call_counts(&self) -> GPUCommandCallCounts {
    let mut counts = self.counts;
    counts += &*self.pass_counts.lock();
    counts
  }

  pub fn finish(self) -> GPUCommandBuffer {
    let counts = self.call_counts();
    let gpu = self.encoder.finish();
    GPUCommandBuffer {
      inner: gpu,
      on_submit: self.on_submit,
      counts,
      _deferred_explicit_destroy: self.deferred_explicit_destroy,
    }
  }

  pub fn with_compute_pass_scoped(mut self, f: impl FnOnce(GPUComputePass)) -> Self {
    f(self.begin_compute_pass());
    self
  }

  pub fn compute_pass_scoped(&mut self, f: impl FnOnce(GPUComputePass)) -> &mut Self {
    f(self.begin_compute_pass());
    self
  }

  pub fn begin_compute_pass(&mut self) -> GPUComputePass {
    self.begin_compute_pass_with_desc(&Default::default())
  }

  pub fn begin_compute_pass_with_desc(
    &mut self,
    desc: &gpu::ComputePassDescriptor,
  ) -> GPUComputePass {
    self.counts.begin_compute_pass += 1;
    GPUComputePass {
      pass: self.encoder.begin_compute_pass(desc).forget_lifetime(),
      placeholder_bg: self.placeholder_bg.clone(),
      counts: Default::default(),
      counts_sink: self.pass_counts.clone(),
    }
  }

  /// the attachments are kept alive by the underlayer render pass, so the description is not
  /// required to outlive the returned pass.
  pub fn begin_render_pass(
    &mut self,
    des: &RenderPassDescription,
    timestamp_writes: Option<gpu::RenderPassTimestampWrites>,
  ) -> GPURenderPass {
    let mut size = None;

    let color_attachments: Vec<_> = des
      .channels
      .iter()
      .map(|(ops, view, resolve_target)| {
        size = Some(view.size());
        Some(gpu::RenderPassColorAttachment {
          view: view.as_render_view(),
          resolve_target: resolve_target.as_ref().map(|t| t.as_render_view()),
          ops: *ops,
          depth_slice: None,
        })
      })
      .collect();

    let depth_stencil_attachment =
      des
        .depth_stencil_target
        .as_ref()
        .map(|(depth_ops, stencil_ops, view)| {
          size = Some(view.size());
          let depth_ops = depth_ops.filter(|_| view.format().has_depth_aspect());
          let stencil_ops = stencil_ops.filter(|_| view.format().has_stencil_aspect());

          gpu::RenderPassDepthStencilAttachment {
            view: view.as_render_view(),
            depth_ops,
            stencil_ops,
          }
        });

    // we do checking here because wgpu validation check is deferred and hard to debug related issue.
    #[cfg(debug_assertions)]
    {
      let iter = des.channels.iter().map(|v| v.1.sample_count());
      let mut _iter = iter.clone();
      let all_equal = match _iter.next() {
        None => true, // Empty iterators are technically all equal
        Some(first) => _iter.all(|x| x == first),
      };

      assert!(all_equal);

      let depth_sample_count = des
        .depth_stencil_target
        .as_ref()
        .map(|v| v.2.sample_count());
      if let (Some(a), Some(b)) = (iter.clone().next(), depth_sample_count) {
        assert_eq!(a, b);
      }
    }

    let desc = gpu::RenderPassDescriptor {
      label: des.name.as_str().into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
      timestamp_writes,
      occlusion_query_set: None,
      multiview_mask: None,
    };

    let formats = RenderTargetFormatsInfo {
      color_formats: des
        .channels
        .iter()
        .map(|(_, view, _)| view.format())
        .collect(),
      depth_stencil_formats: des
        .depth_stencil_target
        .as_ref()
        .map(|(_, _, view)| view.format()),
      sample_count: des
        .channels
        .first()
        .map(|c| &c.1)
        .or_else(|| des.depth_stencil_target.as_ref().map(|c| &c.2))
        .map(|c| c.sample_count())
        .unwrap_or(1),
    };

    self.counts.begin_render_pass += 1;
    let pass = self.encoder.begin_render_pass(&desc).forget_lifetime();
    GPURenderPass {
      pass,
      placeholder_bg: self.placeholder_bg.clone(),
      size: size.expect("attachment should exists"),
      formats,
      counts: Default::default(),
      counts_sink: self.pass_counts.clone(),
    }
  }

  pub fn copy_source_to_texture_2d(
    &mut self,
    device: &GPUDevice,
    source: impl WebGPU2DTextureSource,
    target: &GPU2DTexture,
    origin: (u32, u32),
  ) -> &mut Self {
    let (upload_buffer, size) = source.create_upload_buffer(device);

    self.copy_buffer_to_texture(
      gpu::TexelCopyBufferInfo {
        buffer: &upload_buffer,
        layout: gpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(Into::<usize>::into(size.width) as u32),
          rows_per_image: Some(Into::<usize>::into(size.height) as u32),
        },
      },
      gpu::TexelCopyTextureInfo {
        texture: target.texture.gpu_resource().gpu(),
        mip_level: 0,
        origin: gpu::Origin3d {
          x: origin.0,
          y: origin.1,
          z: 0,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.gpu_size(),
    );
    self
  }
}

/// The wgpu command encoder api wrapper, the calls are counted, see [GPUCommandCallCounts].
impl GPUCommandEncoder {
  /// Access the underlying wgpu command encoder.
  ///
  /// The calls made through the raw encoder are not counted in [GPUCommandCallCounts]. Only use it
  /// when the wgpu type is required, for example to integrate the third party renderer. Any common
  /// usage should use or add the wrapper method in [GPUCommandEncoder].
  pub fn raw_encoder(&mut self) -> &mut gpu::CommandEncoder {
    &mut self.encoder
  }

  pub fn copy_buffer_to_buffer(
    &mut self,
    source: &gpu::Buffer,
    source_offset: gpu::BufferAddress,
    destination: &gpu::Buffer,
    destination_offset: gpu::BufferAddress,
    copy_size: impl Into<Option<gpu::BufferAddress>>,
  ) {
    let copy_size = copy_size.into();
    self.counts.copy_buffer_to_buffer += 1;
    self.counts.copy_buffer_to_buffer_bytes +=
      copy_size.unwrap_or_else(|| source.size().saturating_sub(source_offset));
    self.encoder.copy_buffer_to_buffer(
      source,
      source_offset,
      destination,
      destination_offset,
      copy_size,
    );
  }

  pub fn copy_buffer_to_texture(
    &mut self,
    source: gpu::TexelCopyBufferInfo,
    destination: gpu::TexelCopyTextureInfo,
    copy_size: gpu::Extent3d,
  ) {
    self.counts.copy_buffer_to_texture += 1;
    self.counts.copy_buffer_to_texture_bytes +=
      texel_copy_byte_size(destination.texture.format(), destination.aspect, copy_size);
    self
      .encoder
      .copy_buffer_to_texture(source, destination, copy_size);
  }

  pub fn copy_texture_to_buffer(
    &mut self,
    source: gpu::TexelCopyTextureInfo,
    destination: gpu::TexelCopyBufferInfo,
    copy_size: gpu::Extent3d,
  ) {
    self.counts.copy_texture_to_buffer += 1;
    self.counts.copy_texture_to_buffer_bytes +=
      texel_copy_byte_size(source.texture.format(), source.aspect, copy_size);
    self
      .encoder
      .copy_texture_to_buffer(source, destination, copy_size);
  }

  pub fn copy_texture_to_texture(
    &mut self,
    source: gpu::TexelCopyTextureInfo,
    destination: gpu::TexelCopyTextureInfo,
    copy_size: gpu::Extent3d,
  ) {
    self.counts.copy_texture_to_texture += 1;
    self.counts.copy_texture_to_texture_bytes +=
      texel_copy_byte_size(source.texture.format(), source.aspect, copy_size);
    self
      .encoder
      .copy_texture_to_texture(source, destination, copy_size);
  }

  pub fn clear_texture(
    &mut self,
    texture: &gpu::Texture,
    subresource_range: &gpu::ImageSubresourceRange,
  ) {
    self.counts.clear_texture += 1;
    self.encoder.clear_texture(texture, subresource_range);
  }

  pub fn clear_buffer(
    &mut self,
    buffer: &gpu::Buffer,
    offset: gpu::BufferAddress,
    size: Option<gpu::BufferAddress>,
  ) {
    self.counts.clear_buffer += 1;
    self.counts.clear_buffer_bytes += size.unwrap_or_else(|| buffer.size().saturating_sub(offset));
    self.encoder.clear_buffer(buffer, offset, size);
  }

  pub fn insert_debug_marker(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.encoder.insert_debug_marker(label);
  }

  pub fn push_debug_group(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.encoder.push_debug_group(label);
  }

  pub fn pop_debug_group(&mut self) {
    self.counts.debug_marker += 1;
    self.encoder.pop_debug_group();
  }

  pub fn resolve_query_set(
    &mut self,
    query_set: &gpu::QuerySet,
    query_range: Range<u32>,
    destination: &gpu::Buffer,
    destination_offset: gpu::BufferAddress,
  ) {
    self.counts.resolve_query_set += 1;
    self
      .encoder
      .resolve_query_set(query_set, query_range, destination, destination_offset);
  }

  pub fn write_timestamp(&mut self, query_set: &gpu::QuerySet, query_index: u32) {
    self.counts.write_timestamp += 1;
    self.encoder.write_timestamp(query_set, query_index);
  }

  pub fn build_acceleration_structures<'a>(
    &mut self,
    blas: impl IntoIterator<Item = &'a gpu::BlasBuildEntry<'a>>,
    tlas: impl IntoIterator<Item = &'a gpu::Tlas>,
  ) {
    let mut blas_count = 0;
    let mut tlas_count = 0;
    let blas = blas.into_iter().inspect(|_| blas_count += 1);
    let tlas = tlas.into_iter().inspect(|_| tlas_count += 1);
    self.encoder.build_acceleration_structures(blas, tlas);
    self.counts.build_acceleration_structures += 1;
    self.counts.build_blas_entries += blas_count;
    self.counts.build_tlas_entries += tlas_count;
  }

  /// # Safety
  ///
  /// see [gpu::CommandEncoder::mark_acceleration_structures_built]
  pub unsafe fn mark_acceleration_structures_built<'a>(
    &mut self,
    blas: impl IntoIterator<Item = &'a gpu::Blas>,
    tlas: impl IntoIterator<Item = &'a gpu::Tlas>,
  ) {
    self.counts.mark_acceleration_structures_built += 1;
    unsafe { self.encoder.mark_acceleration_structures_built(blas, tlas) }
  }

  pub fn transition_resources<'a>(
    &mut self,
    buffer_transitions: impl Iterator<Item = wgpu_types::BufferTransition<&'a gpu::Buffer>>,
    texture_transitions: impl Iterator<Item = wgpu_types::TextureTransition<&'a gpu::Texture>>,
  ) {
    self.counts.transition_resources += 1;
    self
      .encoder
      .transition_resources(buffer_transitions, texture_transitions);
  }

  pub fn map_buffer_on_submit<S: core::ops::RangeBounds<gpu::BufferAddress>>(
    &mut self,
    buffer: &gpu::Buffer,
    mode: gpu::MapMode,
    bounds: S,
    callback: impl FnOnce(Result<(), gpu::BufferAsyncError>) + gpu::WasmNotSend + 'static,
  ) {
    self.counts.map_buffer_on_submit += 1;
    self
      .encoder
      .map_buffer_on_submit(buffer, mode, bounds, callback);
  }

  pub fn on_submitted_work_done(&mut self, callback: impl FnOnce() + Send + 'static) {
    self.counts.on_submitted_work_done += 1;
    self.encoder.on_submitted_work_done(callback);
  }
}
