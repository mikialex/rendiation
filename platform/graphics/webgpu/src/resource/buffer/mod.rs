mod uniform;
pub use uniform::*;

mod storage;
pub use storage::*;

use crate::*;

pub type GPUBufferResource = ResourceRc<GPUBuffer>;
pub type GPUBufferResourceView = ResourceViewRc<GPUBuffer>;

impl Resource for GPUBuffer {
  type Descriptor = gpu::BufferUsages;
  type View = GPUBufferView;
  type ViewDescriptor = GPUBufferViewRange;

  fn create_view(&self, des: &Self::ViewDescriptor) -> Self::View {
    GPUBufferView {
      buffer: self.clone(),
      range: *des,
    }
  }
}

impl BindableResourceProvider for GPUBufferResourceView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::Buffer(self.clone())
  }
}

#[derive(Clone)]
pub struct GPUBuffer {
  pub(crate) gpu: Arc<gpu::Buffer>,
  pub(crate) size: std::num::NonZeroU64,
}

impl GPUBuffer {
  pub fn create(device: &GPUDevice, bytes: &[u8], usage: gpu::BufferUsages) -> Self {
    let gpu = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage,
    });
    Self {
      gpu: Arc::new(gpu),
      size: std::num::NonZeroU64::new(bytes.len() as u64).unwrap(),
    }
  }

  pub fn update(&self, queue: &gpu::Queue, bytes: &[u8]) {
    queue.write_buffer(&self.gpu, 0, bytes)
  }

  pub fn gpu(&self) -> &gpu::Buffer {
    &self.gpu
  }
}

impl BindableResourceView for GPUBufferView {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::Buffer(self.as_buffer_binding())
  }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct GPUBufferViewRange {
  /// in bytes
  pub offset: u64,
  /// in bytes, Size of the binding, or None for using the rest of the buffer.
  pub size: Option<std::num::NonZeroU64>,
}

#[derive(Clone)]
pub struct GPUBufferView {
  buffer: GPUBuffer,
  range: GPUBufferViewRange,
}

impl GPUBufferView {
  pub fn as_buffer_binding(&self) -> gpu::BufferBinding {
    gpu::BufferBinding {
      buffer: &self.buffer.gpu,
      offset: self.range.offset,
      size: self.range.size,
    }
  }
}

/// just short convenient method
pub fn create_gpu_buffer(
  data: &[u8],
  usage: gpu::BufferUsages,
  gpu: &GPUDevice,
) -> GPUBufferResource {
  GPUBufferResource::create_with_raw(GPUBuffer::create(gpu, data, usage), usage)
}
