use crate::*;

/// Typed wrapper
pub struct UniformBuffer<T> {
  gpu: wgpu::Buffer,
  phantom: PhantomData<T>,
}

impl<T: Pod> UniformBuffer<T> {
  pub fn create(device: &GPUDevice, data: T) -> Self {
    let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      phantom: PhantomData,
    }
  }

  pub fn update(&self, queue: &wgpu::Queue, data: T) {
    queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[data]))
  }

  pub fn gpu(&self) -> &wgpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResourceView for UniformBuffer<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.gpu.as_entire_binding()
  }
}

/// Typed uniform buffer with cpu data
pub struct UniformBufferData<T> {
  gpu: wgpu::Buffer,
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
    let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu,
      data,
      changed: Cell::new(false),
      last: Default::default(),
    }
  }

  pub fn update(&self, queue: &wgpu::Queue) {
    if self.changed.get() {
      queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[self.data]));
      self.changed.set(false);
      self.last.set(self.data.into());
    }
  }

  pub fn update_with_diff(&self, queue: &wgpu::Queue)
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

  pub fn gpu(&self) -> &wgpu::Buffer {
    &self.gpu
  }
}

impl<T> BindableResourceView for UniformBufferData<T> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    self.gpu.as_entire_binding()
  }
}
