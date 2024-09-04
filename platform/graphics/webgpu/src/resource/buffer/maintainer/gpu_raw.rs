use crate::*;

trait GPULinearStorageImpl {
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_size: u32);
  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder);
  fn raw_gpu(&self) -> &GPUBufferResourceView;
}

pub struct ResizableGPUBuffer<T> {
  gpu: T,
  ctx: GPU,
}

impl<T: LinearStorageBase> LinearStorageBase for ResizableGPUBuffer<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.gpu.max_size()
  }
}

impl<T: GPULinearStorageImpl + LinearStorageBase> ResizeableLinearStorage
  for ResizableGPUBuffer<T>
{
  fn resize(&mut self, new_size: u32) {
    let mut encoder = self.ctx.create_encoder();
    self
      .gpu
      .resize_gpu(&mut encoder, &self.ctx.device, new_size);
    self.ctx.queue.submit_encoder(encoder);
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
    todo!()
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
    todo!()
  }
  fn update_gpu(&mut self, _: &mut GPUCommandEncoder) {}

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    &self.gpu
  }
}
