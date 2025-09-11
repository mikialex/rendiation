use crate::*;

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

impl<T: GPULinearStorage> ResizableLinearStorage for ResizableGPUBuffer<T> {
  fn resize(&mut self, new_size: u32) -> bool {
    let device = self.ctx.device.clone();
    let mut encoder = self.ctx.create_encoder();
    self.abstract_gpu().resize_gpu(
      &mut encoder,
      &device,
      (new_size * std::mem::size_of::<T::Item>() as u32) as u64,
    );
    self.ctx.queue.submit_encoder(encoder);
    true
  }
}

impl<T: GPULinearStorage> GPULinearStorage for ResizableGPUBuffer<T> {
  type GPUType = T;
  fn gpu(&self) -> &Self::GPUType {
    &self.gpu
  }

  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self.gpu.abstract_gpu()
  }
}

impl<T: Std430> LinearStorageBase for AbstractStorageBuffer<[T]> {
  type Item = T;

  fn max_size(&self) -> u32 {
    (self.byte_size() as usize / std::mem::size_of::<T>()) as u32
  }
}

impl<T: Std430> LinearStorageBase for AbstractReadonlyStorageBuffer<[T]> {
  type Item = T;

  fn max_size(&self) -> u32 {
    (self.byte_size() as usize / std::mem::size_of::<T>()) as u32
  }
}

impl<T: Std430> LinearStorageBase for StorageBufferDataView<[T]> {
  type Item = T;
  fn max_size(&self) -> u32 {
    self.item_count()
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> GPULinearStorage for StorageBufferDataView<[T]> {
  type GPUType = Self;
  fn gpu(&self) -> &Self::GPUType {
    self
  }
  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self
  }
}

impl<T: Std430> LinearStorageBase for StorageBufferReadonlyDataView<[T]> {
  type Item = T;
  fn max_size(&self) -> u32 {
    self.item_count()
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> GPULinearStorage for StorageBufferReadonlyDataView<[T]> {
  type GPUType = Self;
  fn gpu(&self) -> &Self::GPUType {
    self
  }
  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self
  }
}
