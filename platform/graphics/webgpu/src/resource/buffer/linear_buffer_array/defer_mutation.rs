use crate::*;

pub struct DeferMutationToGPUUpdate<T: LinearStorageBase> {
  pub inner: T,
  pub updates: FastHashMap<u32, Option<T::Item>>,
}

impl<T> LinearStorageDirectAccess for DeferMutationToGPUUpdate<T>
where
  T: LinearStorageDirectAccess,
{
  fn remove(&mut self, idx: u32) {
    self.updates.insert(idx, None);
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    self.updates.insert(idx, Some(v));
    Some(())
  }
}

impl<T: LinearStorageViewAccess> LinearStorageViewAccess for DeferMutationToGPUUpdate<T> {
  fn view(&self) -> &[Self::Item] {
    self.inner.view()
  }
}

impl<T: ResizableLinearStorage> ResizableLinearStorage for DeferMutationToGPUUpdate<T> {
  fn resize(&mut self, new_size: u32) -> bool {
    self.inner.resize(new_size)
  }
}

impl<T: LinearStorageBase> LinearStorageBase for DeferMutationToGPUUpdate<T> {
  type Item = T::Item;

  fn max_size(&self) -> u32 {
    self.inner.max_size()
  }
}

impl<T: GPULinearStorage + LinearStorageDirectAccess> GPULinearStorage
  for DeferMutationToGPUUpdate<T>
{
  type GPUType = T::GPUType;

  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder) {
    for (idx, v) in self.updates.iter_mut() {
      if let Some(v) = v {
        self.inner.set_value(*idx, *v);
      } else {
        self.inner.remove(*idx);
      }
    }
    self.updates = Default::default();
    self.inner.update_gpu(encoder)
  }

  fn gpu(&self) -> &Self::GPUType {
    self.inner.gpu()
  }

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.inner.raw_gpu()
  }
}
