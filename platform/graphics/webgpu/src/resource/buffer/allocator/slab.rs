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

impl<T: GPULinearStorage> GPULinearStorage for GPUSlatAllocateMaintainer<T> {
  type GPUType = T::GPUType;

  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder) {
    self.buffer.update_gpu(encoder)
  }
  fn gpu(&self) -> &Self::GPUType {
    self.buffer.gpu()
  }
  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.buffer.raw_gpu()
  }
}

pub type StorageBufferSlabAllocatePool<T> = SlabAllocatePool<StorageBufferReadOnlyDataView<[T]>>;
pub type SlabAllocatePool<T> = GPUSlatAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;

pub fn create_storage_buffer_slab_allocate_pool<T: Std430>(
  gpu: &GPU,
  init_size: u32,
  max_size: u32,
) -> StorageBufferSlabAllocatePool<T> {
  let buffer = StorageBufferReadOnlyDataView::<[T]>::create_by(
    &gpu.device,
    StorageBufferInit::Zeroed(NonZeroU64::new(init_size as u64).unwrap()),
  );

  let buffer = create_growable_buffer(gpu, buffer, max_size);
  GPUSlatAllocateMaintainer::new(buffer)
}
