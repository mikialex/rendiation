use crate::*;

pub struct VecWithStorageBuffer<T: LinearStorageBase> {
  pub inner: T,
  pub vec: Vec<T::Item>,
  pub diff: bool,
  pub none_default: T::Item,
}

impl<T: ResizableLinearStorage> ResizableLinearStorage for VecWithStorageBuffer<T>
where
  T::Item: Zeroable,
{
  fn resize(&mut self, new_size: u32) -> bool {
    self.inner.resize(new_size);
    self.vec.resize(new_size as usize, self.none_default);
    true
  }
}

// todo, not panic when out bound
impl<T> LinearStorageDirectAccess for VecWithStorageBuffer<T>
where
  T: LinearStorageDirectAccess,
  T::Item: PartialEq,
{
  fn remove(&mut self, idx: u32) -> Option<()> {
    self.inner.remove(idx)?;
    self.set_value(idx, self.none_default)
  }
  fn removes(&mut self, offset: u32, len: u32) -> Option<()> {
    self.inner.removes(offset, len);
    for i in offset..(offset + len) {
      self.set_value(i, self.none_default)?;
    }
    Some(())
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    if self.diff && self.vec[idx as usize] == v {
      return Some(());
    }
    self.vec[idx as usize] = v;
    self.inner.set_value(idx, v)
  }

  unsafe fn set_value_sub_bytes(
    &mut self,
    idx: u32,
    field_byte_offset: usize,
    v: &[u8],
  ) -> Option<()> {
    let view = self.vec.get_mut(idx as usize)?;
    let view = bytes_of_mut(view);
    let view = view.get_mut(field_byte_offset..(field_byte_offset + v.len()))?;
    if self.diff && view == v {
      return Some(());
    }
    view.copy_from_slice(v);
    self.inner.set_value_sub_bytes(idx, field_byte_offset, v)
  }

  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    let idx = offset as usize;
    let view = self.vec.get_mut(idx..(idx + v.len()))?;
    if self.diff && view == v {
      return Some(());
    }
    view.copy_from_slice(v);
    self.inner.set_values(offset, v)
  }
}

impl<T: LinearStorageBase> LinearStorageViewAccess for VecWithStorageBuffer<T> {
  fn view(&self) -> &[Self::Item] {
    &self.vec
  }
}

impl<T: LinearStorageBase> LinearStorageBase for VecWithStorageBuffer<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.inner.max_size()
  }
}

impl<T: GPULinearStorage + LinearStorageBase> GPULinearStorage for VecWithStorageBuffer<T> {
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
