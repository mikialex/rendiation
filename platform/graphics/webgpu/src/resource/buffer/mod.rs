mod uniform;
pub use uniform::*;

mod storage;
pub use storage::*;

mod allocator;
pub use allocator::*;

mod linear_buffer_array;
pub use linear_buffer_array::*;

mod abstract_resource;
pub use abstract_resource::*;

use crate::*;

pub type GPUBufferResource = ResourceRc<GPUBuffer>;
pub type GPUBufferResourceView = ResourceViewRc<GPUBuffer>;

impl Resource for GPUBuffer {
  type Descriptor = GPUBufferDescriptor;
  type View = GPUBufferView;
  type ViewDescriptor = GPUBufferViewRange;

  fn create_view(&self, des: &Self::ViewDescriptor) -> Self::View {
    GPUBufferView {
      buffer: self.clone(),
      range: *des,
    }
  }
}

pub struct GPUBufferDescriptor {
  pub usage: gpu::BufferUsages,
  pub size: std::num::NonZeroU64,
}

impl BindableResourceProvider for GPUBufferResourceView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::Buffer(self.clone())
  }
}

#[derive(Clone)]
pub struct GPUBuffer {
  pub(crate) gpu: Arc<gpu::Buffer>,
}

pub enum BufferInit<'a> {
  WithInit(&'a [u8]),
  Zeroed(std::num::NonZeroU64),
}

impl BufferInit<'_> {
  pub fn size(&self) -> NonZeroU64 {
    match self {
      BufferInit::WithInit(bytes) => std::num::NonZeroU64::new(bytes.len() as u64).unwrap(),
      BufferInit::Zeroed(size) => *size,
    }
  }
}

impl GPUBuffer {
  pub fn create(
    device: &GPUDevice,
    label: Option<&str>,
    init: BufferInit,
    usage: gpu::BufferUsages,
  ) -> Self {
    let gpu = match init {
      BufferInit::WithInit(bytes) => device.create_buffer_init(&gpu::util::BufferInitDescriptor {
        label,
        contents: bytes,
        usage,
      }),
      BufferInit::Zeroed(size) => device.create_buffer(&wgpu::BufferDescriptor {
        label,
        size: size.get(),
        usage,
        mapped_at_creation: false,
      }),
    };
    Self { gpu: Arc::new(gpu) }
  }

  pub fn update(&self, queue: &gpu::Queue, bytes: &[u8]) {
    queue.write_buffer(&self.gpu, 0, bytes)
  }

  pub fn gpu(&self) -> &gpu::Buffer {
    &self.gpu
  }
}

impl BindableResourceView for GPUBufferView {
  fn as_bindable(&self) -> gpu::BindingResource<'_> {
    gpu::BindingResource::Buffer(self.as_buffer_binding())
  }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct GPUBufferViewRange {
  /// in bytes
  pub offset: u64,
  /// in bytes, Size of the binding, or None for using the rest of the buffer.
  pub size: Option<std::num::NonZeroU64>,
}

#[derive(Clone)]
pub struct GPUBufferView {
  pub buffer: GPUBuffer,
  pub range: GPUBufferViewRange,
}

impl GPUBufferResourceView {
  pub fn view_byte_size(&self) -> NonZeroU64 {
    if let Some(size) = self.range.size {
      size
    } else {
      self.entire_buffer_size()
    }
  }

  pub fn entire_buffer_size(&self) -> NonZeroU64 {
    self.resource.desc.size
  }
}

impl GPUBufferView {
  pub fn as_buffer_binding(&self) -> gpu::BufferBinding<'_> {
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
  device: &GPUDevice,
) -> GPUBufferResource {
  GPUBufferResource::create_with_raw(
    GPUBuffer::create(device, None, BufferInit::WithInit(data), usage),
    GPUBufferDescriptor {
      usage,
      size: NonZeroU64::new(data.len() as u64).unwrap(),
    },
    device,
  )
}

/// just short convenient method
pub fn create_gpu_buffer_zeroed(
  byte_size: u64,
  usage: gpu::BufferUsages,
  device: &GPUDevice,
) -> GPUBufferResource {
  GPUBufferResource::create_with_raw(
    GPUBuffer::create(
      device,
      None,
      BufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
      usage,
    ),
    GPUBufferDescriptor {
      usage,
      size: NonZeroU64::new(byte_size).unwrap(),
    },
    device,
  )
}
