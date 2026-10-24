mod uniform;
pub use uniform::*;

mod storage;
pub use storage::*;

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
    #[cfg(debug_assertions)] // wgpu validation is too late to locate the bug
    {
      if let Some(size) = des.size {
        assert!(des.offset + size.get() <= self.gpu.size());
      }
      assert!(des.offset < self.gpu.size());
    }

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
  pub(crate) gpu: gpu::Buffer,
}

pub enum BufferInit<'a> {
  WithInit(&'a [u8]),
  Zeroed(std::num::NonZeroU64),
}

/// The buffer size used when the requested size is zero. Zero sized buffer is not allowed in many
/// usages (for example the storage binding), so we create a minimal buffer instead.
pub const MIN_BUFFER_BYTE_SIZE: u64 = 4;

/// Return the size as non zero. If the size is zero, a warning is logged and
/// [MIN_BUFFER_BYTE_SIZE] is returned.
pub fn buffer_byte_size_or_min(byte_size: u64, label: Option<&str>) -> NonZeroU64 {
  NonZeroU64::new(byte_size).unwrap_or_else(|| {
    log::warn!(
      "try to create zero sized buffer <{}>, a {MIN_BUFFER_BYTE_SIZE} bytes buffer is created instead",
      label.unwrap_or("unnamed")
    );
    NonZeroU64::new(MIN_BUFFER_BYTE_SIZE).unwrap()
  })
}

impl<'a> BufferInit<'a> {
  /// Return None if the init data is empty.
  pub fn size(&self) -> Option<NonZeroU64> {
    match self {
      BufferInit::WithInit(bytes) => NonZeroU64::new(bytes.len() as u64),
      BufferInit::Zeroed(size) => Some(*size),
    }
  }

  /// Replace the empty init data with a [MIN_BUFFER_BYTE_SIZE] zeroed init, see
  /// [buffer_byte_size_or_min]
  fn ensure_not_empty(self, label: Option<&str>) -> (Self, NonZeroU64) {
    match self.size() {
      Some(size) => (self, size),
      None => {
        let size = buffer_byte_size_or_min(0, label);
        (BufferInit::Zeroed(size), size)
      }
    }
  }
}

impl GPUBuffer {
  /// If the init data is empty, a [MIN_BUFFER_BYTE_SIZE] zeroed buffer is created instead with
  /// a warning.
  pub fn create(
    device: &GPUDevice,
    label: Option<&str>,
    init: BufferInit,
    usage: gpu::BufferUsages,
  ) -> Self {
    let (init, _) = init.ensure_not_empty(label);
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
    Self { gpu }
  }

  /// Create the buffer and wrap it as a resource, see [GPUBuffer::create] for the empty init
  /// handling.
  pub fn create_resource(
    device: &GPUDevice,
    label: Option<&str>,
    init: BufferInit,
    usage: gpu::BufferUsages,
  ) -> GPUBufferResource {
    let (init, size) = init.ensure_not_empty(label);
    let gpu = Self::create(device, label, init, usage);
    GPUBufferResource::create_with_raw(gpu, GPUBufferDescriptor { usage, size }, device)
  }

  pub fn update(&self, queue: &GPUQueue, bytes: &[u8]) {
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
      // if the size is not specified, the view covers the rest of the buffer from the view offset
      NonZeroU64::new(self.entire_buffer_size().get() - self.range.offset).unwrap()
    }
  }

  /// if the view covers the entire buffer
  pub fn is_full_view(&self) -> bool {
    self.range.offset == 0 && self.view_byte_size() == self.entire_buffer_size()
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
///
/// If the data is empty, a [MIN_BUFFER_BYTE_SIZE] zeroed buffer is created instead with a warning.
pub fn create_gpu_buffer(
  data: &[u8],
  usage: gpu::BufferUsages,
  device: &GPUDevice,
) -> GPUBufferResource {
  GPUBuffer::create_resource(device, None, BufferInit::WithInit(data), usage)
}

/// just short convenient method
///
/// If the byte_size is zero, a [MIN_BUFFER_BYTE_SIZE] buffer is created instead with a warning.
pub fn create_gpu_buffer_zeroed(
  byte_size: u64,
  usage: gpu::BufferUsages,
  device: &GPUDevice,
) -> GPUBufferResource {
  let size = buffer_byte_size_or_min(byte_size, None);
  GPUBuffer::create_resource(device, None, BufferInit::Zeroed(size), usage)
}

#[cfg(test)]
#[pollster::test]
async fn test_zero_sized_buffer_creation() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let min = NonZeroU64::new(MIN_BUFFER_BYTE_SIZE).unwrap();
  let usage = gpu::BufferUsages::COPY_DST;

  assert_eq!(create_gpu_buffer(&[], usage, &gpu.device).desc.size, min);
  assert_eq!(
    create_gpu_buffer_zeroed(0, usage, &gpu.device).desc.size,
    min
  );

  let storage = create_gpu_read_write_storage::<[u32]>(ZeroedArrayByArrayLength(0), &gpu, "");
  assert_eq!(storage.gpu.resource.desc.size, min);
}
