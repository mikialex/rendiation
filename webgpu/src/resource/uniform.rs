use crate::*;

pub type UniformBufferView<T> = ResourceViewRc<UniformBuffer<T>>;

impl<T: 'static> Resource for UniformBuffer<T> {
  type Descriptor = ();
  type View = Rc<gpu::Buffer>;
  type ViewDescriptor = ();

  fn create_view(&self, _des: &Self::ViewDescriptor) -> Self::View {
    self.gpu.clone()
  }
}

impl<T: Pod> InitResourceBySource for UniformBuffer<T> {
  type Source = T;

  fn create_resource_with_source(
    source: &Self::Source,
    device: &GPUDevice,
  ) -> (Self, Self::Descriptor) {
    (Self::create(device, *source), ())
  }
}

impl BindableResourceView for Rc<gpu::Buffer> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.as_entire_binding()
  }
}

/// Typed wrapper
pub struct UniformBuffer<T> {
  gpu: Rc<gpu::Buffer>,
  phantom: PhantomData<T>,
}

impl<T: Pod> UniformBuffer<T> {
  pub fn create(device: &GPUDevice, data: T) -> Self {
    let gpu = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu: Rc::new(gpu),
      phantom: PhantomData,
    }
  }

  pub fn update(&self, queue: &gpu::Queue, data: T) {
    queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[data]))
  }

  pub fn gpu(&self) -> &gpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResourceView for UniformBuffer<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_entire_binding()
  }
}

/// Typed uniform buffer with cpu data
pub struct UniformBufferData<T> {
  gpu: gpu::Buffer,
  data: T,
  last: Cell<Option<T>>,
  changed: Cell<bool>,
}

impl<T> Deref for UniformBufferData<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl<T> DerefMut for UniformBufferData<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.changed.set(true);
    &mut self.data
  }
}

impl<T: Pod> UniformBufferData<T> {
  pub fn create_default(device: &GPUDevice) -> Self
  where
    T: Default,
  {
    Self::create(device, T::default())
  }

  pub fn create(device: &GPUDevice, data: T) -> Self {
    let gpu = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      data,
      changed: Cell::new(false),
      last: Default::default(),
    }
  }

  pub fn update(&self, queue: &gpu::Queue) {
    if self.changed.get() {
      queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]));
      self.changed.set(false);
      self.last.set(self.data.into());
    }
  }

  pub fn update_with_diff(&self, queue: &gpu::Queue)
  where
    T: PartialEq,
  {
    if self.changed.get() {
      if let Some(last) = self.last.get() {
        if last != self.data {
          queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]))
        }
        self.last.set(self.data.into());
      } // if last is none, means we use init value, not need update
      self.changed.set(false);
    }
  }

  pub fn gpu(&self) -> &gpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResourceView for UniformBufferData<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_entire_binding()
  }
}
