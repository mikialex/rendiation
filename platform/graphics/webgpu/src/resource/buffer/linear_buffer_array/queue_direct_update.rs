use crate::*;

pub struct GPUStorageDirectQueueUpdate<T> {
  pub queue: GPUQueue,
  pub inner: T,
}

impl<T> LinearStorage for GPUStorageDirectQueueUpdate<T>
where
  T: GPULinearStorage + LinearStorageBase,
  T::Item: Pod,
{
  fn remove(&mut self, _: u32) {
    // do nothing, zeroable when removing behavior can be controlled by upper layer
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    let buffer = self.inner.raw_gpu().resource.gpu();
    let offset = idx * std::mem::size_of::<T::Item>() as u32;
    self.queue.write_buffer(buffer, offset as u64, bytes_of(&v));
    Some(())
  }
  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    let buffer = self.inner.raw_gpu().resource.gpu();
    let offset = offset * std::mem::size_of::<T::Item>() as u32;
    let v = bytemuck::cast_slice(v);
    self.queue.write_buffer(buffer, offset as u64, v);
    Some(())
  }
}

impl<T: LinearStorageBase> LinearStorageBase for GPUStorageDirectQueueUpdate<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.inner.max_size()
  }
}

impl<T: GPULinearStorage> GPULinearStorage for GPUStorageDirectQueueUpdate<T> {
  type GPUType = T::GPUType;

  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder) {
    self.inner.update_gpu(encoder)
  }

  fn gpu(&self) -> &Self::GPUType {
    self.inner.gpu()
  }

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.inner.raw_gpu()
  }
}

impl<T: ResizeableLinearStorage> ResizeableLinearStorage for GPUStorageDirectQueueUpdate<T> {
  fn resize(&mut self, new_size: u32) {
    self.inner.resize(new_size)
  }
}
