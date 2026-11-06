use crate::*;

#[derive(Clone)]
pub struct GPUQueue {
  inner: wgpu::Queue,
  counters: Arc<GPUCallCounters>,
}

impl GPUQueue {
  pub(crate) fn new(queue: wgpu::Queue, device: &GPUDevice) -> Self {
    Self {
      inner: queue,
      counters: device.inner.counters.clone(),
    }
  }

  pub fn submit_encoder(&self, encoder: GPUCommandEncoder) {
    let cmd = encoder.finish();
    self.submit_command_buffers(std::iter::once(cmd));
  }

  /// the command buffers are submitted in one wgpu submit call
  pub fn submit_command_buffers(
    &self,
    command_buffers: impl IntoIterator<Item = GPUCommandBuffer>,
  ) -> gpu::SubmissionIndex {
    let mut submitted = Vec::new();
    let raw: Vec<_> = command_buffers
      .into_iter()
      .map(|cmd| {
        self.counters.submitted_commands.merge(&cmd.counts);
        submitted.push((cmd.on_submit, cmd._deferred_explicit_destroy));
        cmd.inner
      })
      .collect();

    let device_counters = &self.counters.device;
    device_counters.queue_submit.inc();
    device_counters
      .queue_submit_command_buffers
      .add(raw.len() as u64);
    let index = self.inner.submit(raw);

    for (on_submit, _deferred_explicit_destroy) in submitted {
      on_submit.notify();
    }
    index
  }

  /// Hand `cmd` to a submit this queue does not make, for example a third party renderer that
  /// submits it with its own command buffers in one wgpu submit call. Its calls count as
  /// submitted now. Keep the returned guard until that submit happened: dropping it notifies the
  /// buffer's submit waiters, and on the web runs the destroys deferred while it recorded.
  pub fn hand_over(&self, cmd: GPUCommandBuffer) -> (gpu::CommandBuffer, HandedOverCommandBuffer) {
    self.counters.submitted_commands.merge(&cmd.counts);
    let guard = HandedOverCommandBuffer {
      on_submit: cmd.on_submit,
      _deferred_explicit_destroy: cmd._deferred_explicit_destroy,
    };
    (cmd.inner, guard)
  }
}

/// What a command buffer handed over by [`GPUQueue::hand_over`] holds until its submit.
pub struct HandedOverCommandBuffer {
  on_submit: OnSubmitNotifier,
  _deferred_explicit_destroy: CommandBufferDeferExplicitDestroyFlusher,
}

impl Drop for HandedOverCommandBuffer {
  fn drop(&mut self) {
    std::mem::take(&mut self.on_submit).notify();
  }
}

/// The wgpu queue api wrapper, the calls are counted, see [GPUCallCountReport].
impl GPUQueue {
  /// Access the underlying wgpu queue.
  ///
  /// The calls made through the raw queue are not counted in [GPUCallCountReport]. Only use it when
  /// the wgpu type is required, for example to integrate the third party renderer. Any common usage
  /// should use or add the wrapper method in [GPUQueue].
  pub fn raw_queue(&self) -> &gpu::Queue {
    &self.inner
  }

  pub fn write_buffer(&self, buffer: &gpu::Buffer, offset: gpu::BufferAddress, data: &[u8]) {
    let counters = &self.counters.device;
    counters.queue_write_buffer.inc();
    counters.queue_write_buffer_bytes.add(data.len() as u64);
    self.inner.write_buffer(buffer, offset, data)
  }

  pub fn write_buffer_with(
    &self,
    buffer: &gpu::Buffer,
    offset: gpu::BufferAddress,
    size: gpu::BufferSize,
  ) -> Option<gpu::QueueWriteBufferView> {
    let counters = &self.counters.device;
    counters.queue_write_buffer_with.inc();
    counters.queue_write_buffer_with_bytes.add(size.get());
    self.inner.write_buffer_with(buffer, offset, size)
  }

  pub fn write_texture(
    &self,
    texture: gpu::TexelCopyTextureInfo,
    data: &[u8],
    data_layout: gpu::TexelCopyBufferLayout,
    size: gpu::Extent3d,
  ) {
    let counters = &self.counters.device;
    counters.queue_write_texture.inc();
    counters.queue_write_texture_bytes.add(data.len() as u64);
    self.inner.write_texture(texture, data, data_layout, size)
  }

  #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
  pub fn copy_external_image_to_texture(
    &self,
    source: &wgpu_types::CopyExternalImageSourceInfo,
    dest: wgpu_types::CopyExternalImageDestInfo<&gpu::Texture>,
    size: gpu::Extent3d,
  ) {
    self
      .counters
      .device
      .queue_copy_external_image_to_texture
      .inc();
    self
      .inner
      .copy_external_image_to_texture(source, dest, size)
  }

  pub fn get_timestamp_period(&self) -> f32 {
    self.inner.get_timestamp_period()
  }

  pub fn on_submitted_work_done(&self, callback: impl FnOnce() + Send + 'static) {
    self.inner.on_submitted_work_done(callback)
  }

  pub fn compact_blas(&self, blas: &gpu::Blas) -> gpu::Blas {
    self.counters.device.queue_compact_blas.inc();
    self.inner.compact_blas(blas)
  }
}
