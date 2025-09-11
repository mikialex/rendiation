use crate::*;

pub struct GPUSlatAllocateMaintainer<T> {
  used_count: u32,
  allocator: slab::Slab<()>,
  buffer: T,
}

impl<T: LinearStorageBase> GPUSlatAllocateMaintainer<T> {
  pub fn new(buffer: T) -> Self {
    Self {
      used_count: 0,
      allocator: slab::Slab::with_capacity(buffer.max_size() as usize),
      buffer,
    }
  }
}

impl<T: LinearStorageBase + LinearStorageDirectAccess> LinearAllocatorStorage
  for GPUSlatAllocateMaintainer<T>
{
  fn deallocate(&mut self, idx: u32) {
    self.allocator.remove(idx as usize);
    self.buffer.remove(idx);
  }

  fn allocate_value(&mut self, v: Self::Item) -> Option<u32> {
    let idx = self.allocator.insert(()) as u32;
    self.buffer.set_value(idx, v)?; // the under layer should handle the resize and propagate resize failure
    Some(idx)
  }

  fn deallocate_back(&mut self, idx: u32) -> Option<Self::Item>
  where
    Self: LinearStorageViewAccess,
  {
    let value = *self.get(idx)?;
    self.deallocate_back(idx);
    Some(value)
  }
}

impl<T: LinearStorageBase> LinearStorageBase for GPUSlatAllocateMaintainer<T> {
  type Item = T::Item;
  fn max_size(&self) -> u32 {
    self.buffer.max_size()
  }
}

impl<T: LinearStorageBase> AllocatorStorageBase for GPUSlatAllocateMaintainer<T> {
  fn current_used(&self) -> u32 {
    self.used_count
  }
}

impl<T: LinearStorageViewAccess> LinearStorageViewAccess for GPUSlatAllocateMaintainer<T> {
  fn view(&self) -> &[Self::Item] {
    self.buffer.view()
  }
}

impl<T: LinearStorageDirectAccess> LinearStorageDirectAccess for GPUSlatAllocateMaintainer<T> {
  fn remove(&mut self, idx: u32) -> Option<()> {
    self.buffer.remove(idx)
  }
  fn removes(&mut self, offset: u32, len: u32) -> Option<()> {
    self.buffer.removes(offset, len)
  }
  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
    self.buffer.set_value(idx, v)
  }
  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    self.buffer.set_values(offset, v)
  }
  unsafe fn set_value_sub_bytes(&mut self, idx: u32, field_offset: usize, v: &[u8]) -> Option<()> {
    self.buffer.set_value_sub_bytes(idx, field_offset, v)
  }
}

impl<T: GPULinearStorage> GPULinearStorage for GPUSlatAllocateMaintainer<T> {
  type GPUType = T::GPUType;

  fn gpu(&self) -> &Self::GPUType {
    self.buffer.gpu()
  }

  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self.buffer.abstract_gpu()
  }
}

pub type StorageBufferSlabAllocatePool<T> = SlabAllocatePool<StorageBufferReadonlyDataView<[T]>>;
pub type SlabAllocatePool<T> = GPUSlatAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;

pub fn create_storage_buffer_slab_allocate_pool<T: Std430 + ShaderSizedValueNodeType>(
  gpu: &GPU,
  init_item_count: u32,
  max_item_count: u32,
) -> StorageBufferSlabAllocatePool<T> {
  assert!(max_item_count >= init_item_count);
  let buffer = StorageBufferReadonlyDataView::<[T]>::create_by(
    &gpu.device,
    None,
    ZeroedArrayByArrayLength(init_item_count as usize).into(),
  );

  let buffer = create_growable_buffer(gpu, buffer, max_item_count);
  GPUSlatAllocateMaintainer::new(buffer)
}
