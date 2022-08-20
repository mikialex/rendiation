use shadergraph::Std140;

use crate::*;

pub type UniformBufferResource<T> = ResourceRc<UniformBuffer<T>>;
pub type UniformBufferView<T> = ResourceViewRc<UniformBuffer<T>>;

impl<T: Std140> Resource for UniformBuffer<T> {
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
pub struct UniformBuffer<T: Std140> {
  gpu: Rc<gpu::Buffer>,
  phantom: PhantomData<T>,
}

impl<T: Std140> UniformBuffer<T> {
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

impl<T: Std140> BindableResourceView for UniformBuffer<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_entire_binding()
  }
}

/// Typed uniform buffer with cpu data cache, which could being diffed when updating
pub struct UniformBufferData<T: Std140> {
  gpu: Rc<gpu::Buffer>,
  data: RefCell<T>,
  last: Cell<Option<T>>,
  changed: Cell<bool>,
}

pub type UniformBufferDataResource<T> = ResourceRc<UniformBufferData<T>>;
pub type UniformBufferDataView<T> = ResourceViewRc<UniformBufferData<T>>;

impl<T: Std140> Resource for UniformBufferData<T> {
  type Descriptor = ();
  type View = Rc<gpu::Buffer>;
  type ViewDescriptor = ();

  fn create_view(&self, _des: &Self::ViewDescriptor) -> Self::View {
    self.gpu.clone()
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
    let gpu = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: bytemuck::cast_slice(&[data]),
      usage: gpu::BufferUsages::UNIFORM | gpu::BufferUsages::COPY_DST,
    });
    Self {
      gpu: Rc::new(gpu),
      data: RefCell::new(data),
      changed: Cell::new(false),
      last: Default::default(),
    }
  }

  pub fn mutate(&self, f: impl Fn(&mut T)) {
    let mut data = self.data.borrow_mut();
    f(&mut data);
    self.changed.set(true);
  }

  pub fn update(&self, queue: &gpu::Queue) {
    if self.changed.get() {
      let data = self.data.borrow();
      let data: &T = &data;
      queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[*data]));
      self.changed.set(false);
      self.last.set(Some(*data));
    }
  }

  pub fn update_with_diff(&self, queue: &gpu::Queue)
  where
    T: PartialEq,
  {
    if self.changed.get() {
      if let Some(last) = self.last.get() {
        let data = self.data.borrow();
        let data: &T = &data;
        if last != *data {
          queue.write_buffer(&self.gpu, 0, bytemuck::cast_slice(&[*data]))
        }
        self.last.set(Some(*data));
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
