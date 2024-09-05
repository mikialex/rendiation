use crate::*;

pub struct DeferMutationToGPUUpdate<T: LinearStorageBase> {
  pub inner: T,
  pub(crate) updates: FastHashMap<u32, Option<ItemDelta<T::Item>>>,
  pub(crate) bump_bytes: Vec<u8>,
}

pub(crate) enum ItemDelta<T> {
  Partial {
    offset: usize,
    size: usize,
    field_byte_offset: usize,
  },
  Full(T),
}

impl<T> LinearStorageDirectAccess for DeferMutationToGPUUpdate<T>
where
  T: LinearStorageDirectAccess,
{
  fn remove(&mut self, idx: u32) -> Option<()> {
    self.updates.insert(idx, None);
    Some(())
  }

  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    self.updates.insert(idx, Some(ItemDelta::Full(v)));
    Some(())
  }

  unsafe fn set_value_sub_bytes(
    &mut self,
    idx: u32,
    field_byte_offset: usize,
    v: &[u8],
  ) -> Option<()> {
    let offset = self.bump_bytes.len();
    self.bump_bytes.extend(v);
    self.updates.insert(
      idx,
      Some(ItemDelta::Partial {
        offset,
        size: v.len(),
        field_byte_offset,
      }),
    );
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
        match v {
          ItemDelta::Partial {
            offset,
            size,
            field_byte_offset,
          } => unsafe {
            let v = &self.bump_bytes[*offset..(*offset + *size)];
            self
              .inner
              .set_value_sub_bytes(*idx, *field_byte_offset, v)
              .unwrap();
          },
          ItemDelta::Full(v) => {
            self.inner.set_value(*idx, *v).unwrap();
          }
        }
      } else {
        self.inner.remove(*idx);
      }
    }
    self.updates = Default::default();
    self.bump_bytes = Default::default();
    self.inner.update_gpu(encoder)
  }

  fn gpu(&self) -> &Self::GPUType {
    self.inner.gpu()
  }

  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.inner.raw_gpu()
  }
}
