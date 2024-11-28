use crate::*;

pub trait GPULinearStorageImpl: LinearStorageBase {
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_size: u32);
  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder);
  fn raw_gpu(&self) -> &GPUBufferResourceView;
}

pub struct ResizableGPUBuffer<T> {
  pub gpu: T,
  pub ctx: GPU,
}

impl<T: LinearStorageBase> LinearStorageBase for ResizableGPUBuffer<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.gpu.max_size()
  }
}

impl<T: GPULinearStorageImpl + LinearStorageBase> ResizableLinearStorage for ResizableGPUBuffer<T> {
  fn resize(&mut self, new_size: u32) -> bool {
    let mut encoder = self.ctx.create_encoder();
    self
      .gpu
      .resize_gpu(&mut encoder, &self.ctx.device, new_size);
    self.ctx.queue.submit_encoder(encoder);
    true
  }
}

impl<T: GPULinearStorageImpl> GPULinearStorage for ResizableGPUBuffer<T> {
  type GPUType = T;

  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder) {
    self.gpu.update_gpu(encoder);
  }

  fn gpu(&self) -> &Self::GPUType {
    &self.gpu
  }

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.gpu.raw_gpu()
  }
}

impl<T: Std430> LinearStorageBase for StorageBufferDataView<[T]> {
  type Item = T;
  fn max_size(&self) -> u32 {
    self.item_count()
  }
}

impl<T: Std430> GPULinearStorageImpl for StorageBufferDataView<[T]> {
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_size: u32) {
    self.gpu = resize_impl(
      &self.gpu,
      encoder,
      device,
      new_size * std::mem::size_of::<u32>() as u32,
    );
  }
  fn update_gpu(&mut self, _: &mut GPUCommandEncoder) {}

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    &self.gpu
  }
}

impl<T: Std430> LinearStorageBase for StorageBufferReadOnlyDataView<[T]> {
  type Item = T;
  fn max_size(&self) -> u32 {
    self.item_count()
  }
}

impl<T: Std430> GPULinearStorageImpl for StorageBufferReadOnlyDataView<[T]> {
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_size: u32) {
    self.gpu = resize_impl(
      &self.gpu,
      encoder,
      device,
      new_size * std::mem::size_of::<u32>() as u32,
    );
  }
  fn update_gpu(&mut self, _: &mut GPUCommandEncoder) {}

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    &self.gpu
  }
}

pub struct TypedGPUBuffer<T> {
  pub gpu: GPUBufferResourceView,
  pub(crate) ty: PhantomData<T>,
}

impl<T> TypedGPUBuffer<T> {
  pub fn new(gpu: GPUBufferResourceView) -> Self {
    Self {
      gpu,
      ty: PhantomData,
    }
  }
}

impl<T> LinearStorageBase for TypedGPUBuffer<T> {
  type Item = u32;
  fn max_size(&self) -> u32 {
    let size: u64 = self.gpu.view_byte_size().into();
    let count = size / std::mem::size_of::<u32>() as u64;
    count as u32
  }
}

impl<T> GPULinearStorageImpl for TypedGPUBuffer<T> {
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_size: u32) {
    self.gpu = resize_impl(
      &self.gpu,
      encoder,
      device,
      new_size * std::mem::size_of::<u32>() as u32,
    );
  }

  fn update_gpu(&mut self, _: &mut GPUCommandEncoder) {}

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    &self.gpu
  }
}

fn resize_impl(
  buffer: &GPUBufferResourceView,
  encoder: &mut GPUCommandEncoder,
  device: &GPUDevice,
  new_size: u32,
) -> GPUBufferResourceView {
  let usage = buffer.resource.desc.usage;
  let new_buffer = create_gpu_buffer_zeroed(new_size as u64, usage, device).create_default_view();

  encoder.copy_buffer_to_buffer(
    &buffer.resource.gpu,
    0,
    &new_buffer.resource.gpu,
    0,
    buffer.resource.desc.size.into(),
  );

  new_buffer
}
