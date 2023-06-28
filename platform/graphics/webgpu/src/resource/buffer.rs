use shadergraph::Std140;

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
  pub fn as_buffer_binding(&self) -> BufferBinding {
    BufferBinding {
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

/// Typed uniform buffer with cpu data cache, which could being diffed when updating
#[derive(Clone)]
pub struct UniformBufferDataView<T: Std140> {
  gpu: GPUBufferResourceView,
  diff: Arc<RwLock<DiffState<T>>>,
}

/// just short convenient method
pub fn create_uniform<T: Std140>(data: T, gpu: &GPU) -> UniformBufferDataView<T> {
  UniformBufferDataView::create(&gpu.device, data)
}
pub fn create_uniform2<T: Std140>(data: T, device: &GPUDevice) -> UniformBufferDataView<T> {
  UniformBufferDataView::create(device, data)
}

impl<T: Std140> BindableResourceProvider for UniformBufferDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std140> CacheAbleBindingSource for UniformBufferDataView<T> {
  fn get_uniform(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_uniform()
  }
}
impl<T: Std140> BindableResourceView for UniformBufferDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<T: Std140> UniformBufferDataView<T> {
  pub fn create_default(device: &GPUDevice) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &GPUDevice, data: T) -> Self {
    let usage = gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST;
    let gpu = GPUBuffer::create(device, bytemuck::cast_slice(&[data]), usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, usage).create_default_view();

    Self {
      gpu,
      diff: Arc::new(RwLock::new(DiffState::new(data))),
    }
  }

  pub fn mutate(&self, f: impl Fn(&mut T)) -> &Self {
    let mut state = self.diff.write().unwrap();
    f(&mut state.data);
    state.changed = true;
    self
  }

  pub fn copy_cpu(&self, other: &Self) -> &Self {
    let mut state = self.diff.write().unwrap();
    state.data = other.get();
    state.changed = true;
    self
  }

  pub fn get(&self) -> T {
    self.diff.read().unwrap().data
  }

  pub fn set(&self, v: T) {
    let mut state = self.diff.write().unwrap();
    state.data = v;
    state.changed = true;
  }

  pub fn upload(&self, queue: &gpu::Queue) {
    let mut state = self.diff.write().unwrap();
    if state.changed {
      let data = state.data;
      queue.write_buffer(&self.gpu.resource.gpu, 0, bytemuck::cast_slice(&[data]));
      state.changed = false;
      state.last = Some(data);
    }
  }

  pub fn upload_with_diff(&self, queue: &gpu::Queue)
  where
    T: PartialEq,
  {
    let mut state = self.diff.write().unwrap();
    if state.changed {
      let data = state.data;
      let should_update;

      // if last is none, means we use init value, not need update
      if let Some(last) = state.last {
        should_update = last != data;
        state.last = Some(data);
      } else {
        should_update = true;
      }

      if should_update {
        queue.write_buffer(&self.gpu.resource.gpu, 0, bytemuck::cast_slice(&[data]))
      }

      state.changed = false;
    }
  }
}

struct DiffState<T> {
  data: T,
  last: Option<T>,
  changed: bool,
}

impl<T> DiffState<T> {
  pub fn new(data: T) -> Self {
    Self {
      data,
      last: None,
      changed: false,
    }
  }
}
