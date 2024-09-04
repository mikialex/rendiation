use crate::*;

pub struct ResizeInput {
  pub current_size: u32,
  pub required_size: u32,
}

/// control the grow behavior
///
/// try auto grow when unbound mutation occurred
pub struct CustomGrowBehaviorMaintainer<T> {
  pub inner: T,
  pub size_adjust: Box<dyn Fn(ResizeInput) -> Option<u32>>,
}

impl<T: ResizableLinearStorage> ResizableLinearStorage for CustomGrowBehaviorMaintainer<T> {
  fn resize(&mut self, new_size: u32) -> bool {
    self.check_resize(new_size).is_some()
  }
}

impl<T> CustomGrowBehaviorMaintainer<T>
where
  T: LinearStorageBase + ResizableLinearStorage,
{
  fn check_resize(&mut self, required: u32) -> Option<()> {
    if self.max_size() < required {
      let new_size = (self.size_adjust)(ResizeInput {
        current_size: self.max_size(),
        required_size: required,
      })?;
      return self.inner.resize(new_size).then_some(());
    }
    Some(())
  }
}

impl<T> LinearStorageDirectAccess for CustomGrowBehaviorMaintainer<T>
where
  T: LinearStorageDirectAccess + ResizableLinearStorage,
{
  fn remove(&mut self, idx: u32) {
    self.inner.remove(idx);
  }
  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    let required = idx + 1;
    self.check_resize(required)?;
    self.inner.set_value(idx, v)
  }

  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    let required = offset + v.len() as u32;
    self.check_resize(required)?;
    self.inner.set_values(offset, v)
  }
}

impl<T: LinearStorageBase> LinearStorageBase for CustomGrowBehaviorMaintainer<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.inner.max_size()
  }
}

impl<T: GPULinearStorage> GPULinearStorage for CustomGrowBehaviorMaintainer<T> {
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
