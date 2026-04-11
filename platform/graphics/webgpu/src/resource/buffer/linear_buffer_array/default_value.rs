use crate::*;

pub struct BufferWidthDefaultValue<T: LinearStorageBase> {
  inner: T,
  default_value: T::Item,
}

impl<T: LinearStorageBase + LinearStorageDirectAccess> BufferWidthDefaultValue<T> {
  pub fn new_with_default_init_write(mut inner: T, default_value: T::Item) -> Self {
    let init_size = inner.max_size();
    if init_size > 0 {
      inner
        .set_values(0, &vec![default_value; init_size as usize])
        .expect("BufferWidthDefaultValue init write failed");
    }

    Self {
      inner,
      default_value,
    }
  }
}

impl<T: ResizableLinearStorage + LinearStorageDirectAccess> ResizableLinearStorage
  for BufferWidthDefaultValue<T>
{
  fn resize(&mut self, new_size: u32) -> bool {
    let previous_size = self.inner.max_size();
    let is_grow = new_size > previous_size;
    let success = self.inner.resize(new_size);

    if success && is_grow {
      let write_success = self
        .inner
        .set_values(
          previous_size,
          &vec![self.default_value; (new_size - previous_size) as usize],
        )
        .is_some();

      if !write_success {
        return false;
      }
    }

    success
  }
}

impl<T> LinearStorageDirectAccess for BufferWidthDefaultValue<T>
where
  T: LinearStorageDirectAccess,
{
  fn remove(&mut self, idx: u32) -> Option<()> {
    self.inner.remove(idx)
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    self.inner.set_value(idx, v)
  }

  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    self.inner.set_values(offset, v)
  }

  unsafe fn set_value_sub_bytes(
    &mut self,
    idx: u32,
    field_byte_offset: usize,
    v: &[u8],
  ) -> Option<()> {
    self.inner.set_value_sub_bytes(idx, field_byte_offset, v)
  }
}

impl<T: LinearStorageBase> LinearStorageBase for BufferWidthDefaultValue<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.inner.max_size()
  }
}

impl<T: GPULinearStorage> GPULinearStorage for BufferWidthDefaultValue<T> {
  type GPUType = T::GPUType;
  fn gpu(&self) -> &Self::GPUType {
    self.inner.gpu()
  }

  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self.inner.abstract_gpu()
  }
}
