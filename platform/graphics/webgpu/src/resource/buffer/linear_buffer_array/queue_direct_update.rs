use crate::*;

pub struct GPUStorageDirectQueueUpdate<T> {
  pub queue: GPUQueue,
  pub inner: T,
}

impl<T> LinearStorageDirectAccess for GPUStorageDirectQueueUpdate<T>
where
  T: GPULinearStorage + LinearStorageBase,
  T::Item: Pod,
{
  fn remove(&mut self, _: u32) -> Option<()> {
    // do nothing, zeroable when removing behavior can be controlled by upper layer
    Some(())
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    unsafe { self.set_value_sub_bytes(idx, 0, bytes_of(&v)) }
  }
  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    let v = bytemuck::cast_slice(v);
    unsafe { self.set_value_sub_bytes(offset, 0, v) }
  }
  unsafe fn set_value_sub_bytes(
    &mut self,
    idx: u32,
    field_byte_offset: usize,
    v: &[u8],
  ) -> Option<()> {
    let buffer = self.inner.raw_gpu().resource.gpu();
    let offset = idx as usize * std::mem::size_of::<T::Item>() + field_byte_offset;
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

impl<T: ResizableLinearStorage> ResizableLinearStorage for GPUStorageDirectQueueUpdate<T> {
  fn resize(&mut self, new_size: u32) -> bool {
    self.inner.resize(new_size)
  }
}
