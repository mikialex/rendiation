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

#[derive(Clone)]
pub struct GPUBuffer {
  pub gpu: Rc<gpu::Buffer>,
}

impl GPUBuffer {
  pub fn create(device: &GPUDevice, bytes: &[u8], usage: gpu::BufferUsages) -> Self {
    let gpu = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: bytes,
      usage,
    });
    Self { gpu: Rc::new(gpu) }
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
    gpu::BindingResource::Buffer(BufferBinding {
      buffer: &self.buffer.gpu,
      offset: self.range.offset,
      size: self.range.size,
    })
  }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct GPUBufferViewRange {
  /// in bytes
  pub offset: u64,
  /// in bytes, Size of the binding, or None for using the rest of the buffer.
  pub size: Option<std::num::NonZeroU64>,
}

pub struct GPUBufferView {
  buffer: GPUBuffer,
  range: GPUBufferViewRange,
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
pub struct UniformBufferData<T: Std140> {
  gpu: GPUBuffer,
  data: RefCell<T>,
  last: Cell<Option<T>>,
  changed: Cell<bool>,
}

pub type UniformBufferDataResource<T> = ResourceRc<UniformBufferData<T>>;
pub type UniformBufferDataView<T> = ResourceViewRc<UniformBufferData<T>>;

/// just short convenient method
pub fn create_uniform<T: Std140>(data: T, gpu: &GPU) -> UniformBufferDataView<T> {
  UniformBufferDataResource::create_with_source(data, &gpu.device).create_default_view()
}

impl<T: Std140> Resource for UniformBufferData<T> {
  type Descriptor = ();
  type View = GPUBufferView;
  type ViewDescriptor = ();

  fn create_view(&self, _des: &Self::ViewDescriptor) -> Self::View {
    GPUBufferView {
      buffer: self.gpu.clone(),
      range: GPUBufferViewRange {
        offset: 0,
        size: None,
      },
    }
  }
}

impl<T: Std140> InitResourceBySource for UniformBufferData<T> {
  type Source = T;

  fn create_resource_with_source(
    source: &Self::Source,
    device: &GPUDevice,
  ) -> (Self, Self::Descriptor) {
    (Self::create(device, *source), ())
  }
}

impl<T: Std140> UniformBufferData<T> {
  pub fn create_default(device: &GPUDevice) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &GPUDevice, data: T) -> Self {
    let gpu = GPUBuffer::create(
      device,
      bytemuck::cast_slice(&[data]),
      gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST,
    );

    Self {
      gpu,
      data: RefCell::new(data),
      changed: Cell::new(false),
      last: Default::default(),
    }
  }

  pub fn mutate(&self, f: impl Fn(&mut T)) -> &Self {
    let mut data = self.data.borrow_mut();
    f(&mut data);
    self.changed.set(true);
    self
  }

  pub fn copy_cpu(&self, other: &Self) -> &Self {
    let mut data = self.data.borrow_mut();
    *data = *other.data.borrow();
    self.changed.set(true);
    self
  }

  pub fn get(&self) -> T {
    *self.data.borrow()
  }

  pub fn set(&self, v: T) {
    let mut data = self.data.borrow_mut();
    *data = v;
    self.changed.set(true);
  }

  pub fn upload(&self, queue: &gpu::Queue) {
    if self.changed.get() {
      let data = self.data.borrow();
      let data: &T = &data;
      queue.write_buffer(&self.gpu.gpu, 0, bytemuck::cast_slice(&[*data]));
      self.changed.set(false);
      self.last.set(Some(*data));
    }
  }

  pub fn upload_with_diff(&self, queue: &gpu::Queue)
  where
    T: PartialEq,
  {
    if self.changed.get() {
      if let Some(last) = self.last.get() {
        let data = self.data.borrow();
        let data: &T = &data;
        if last != *data {
          queue.write_buffer(&self.gpu.gpu, 0, bytemuck::cast_slice(&[*data]))
        }
        self.last.set(Some(*data));
      } // if last is none, means we use init value, not need update
      self.changed.set(false);
    }
  }
}
