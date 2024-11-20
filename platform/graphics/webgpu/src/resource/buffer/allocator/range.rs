use crate::*;

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GPURangeAllocateMaintainer<T> {
  used_count: u32,
  // todo, remove this if we can get offset from handle in allocator
  // offset => (size, handle)
  ranges: FastHashMap<u32, (u32, AllocationHandle)>,
  // todo, try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
  buffer: T,
  gpu: GPU,
}

impl<T> GPURangeAllocateMaintainer<T>
where
  T: ResizableLinearStorage + GPULinearStorage + LinearStorageDirectAccess,
{
  pub fn new(gpu: &GPU, buffer: T) -> Self {
    Self {
      allocator: xalloc::SysTlsf::new(buffer.max_size()),
      buffer,
      gpu: gpu.clone(),
      used_count: 0,
      ranges: Default::default(),
    }
  }

  /// return if grow success
  fn relocate(
    &mut self,
    new_size: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> bool {
    // resize the underlayer buffer
    let origin_gpu_buffer = self.buffer.raw_gpu().clone();
    if !self.buffer.resize(new_size) {
      return false;
    }
    let new_gpu_buffer = self.buffer.raw_gpu().clone();

    // move the data

    let mut encoder = self.gpu.device.create_encoder();
    let mut new_allocator = xalloc::SysTlsf::new(new_size);

    // make sure any pending mutation is applied
    self.buffer.update_gpu(&mut encoder);

    let item_byte_width = std::mem::size_of::<T::Item>() as u32;
    let mut new_ranges = FastHashMap::default();
    self.ranges.iter_mut().for_each(|(offset, (size, _))| {
      let (new_token, new_offset) = new_allocator
        .alloc(*size)
        .expect("relocation should success");

      encoder.copy_buffer_to_buffer(
        origin_gpu_buffer.resource.gpu(),
        *offset as u64 * item_byte_width as u64,
        new_gpu_buffer.resource.gpu(),
        new_offset as u64 * item_byte_width as u64,
        *size as u64 * item_byte_width as u64,
      );

      new_ranges.insert(new_offset, (*size, new_token));
      relocation_handler(RelocationMessage {
        previous_offset: *offset,
        new_offset,
      })
    });

    self.gpu.queue.submit_encoder(encoder);

    self.allocator = new_allocator;
    self.ranges = new_ranges;

    true
  }

  fn allocate_range_impl(
    &mut self,
    count: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32>
  where
    Self: LinearStorageDirectAccess,
  {
    assert!(count > 0);
    loop {
      if let Some((token, offset)) = self.allocator.alloc(count) {
        self.ranges.insert(offset, (count, token));
        self.used_count += count;

        break Some(offset);
      } else if !self.relocate(self.buffer.max_size() + count, relocation_handler) {
        return None;
      }
    }
  }
}

impl<T: LinearStorageBase> LinearStorageBase for GPURangeAllocateMaintainer<T> {
  type Item = T::Item;
  fn max_size(&self) -> u32 {
    self.buffer.max_size()
  }
}

impl<T: LinearStorageDirectAccess> LinearStorageDirectAccess for GPURangeAllocateMaintainer<T> {
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

impl<T: GPULinearStorage> GPULinearStorage for GPURangeAllocateMaintainer<T> {
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

impl<T: LinearStorageBase> AllocatorStorageBase for GPURangeAllocateMaintainer<T> {
  fn current_used(&self) -> u32 {
    self.used_count
  }
}

impl<T> RangeAllocatorStorage for GPURangeAllocateMaintainer<T>
where
  T: ResizableLinearStorage + LinearStorageDirectAccess + GPULinearStorage,
{
  fn deallocate(&mut self, offset: u32) {
    let (size, token) = self.ranges.remove(&offset).unwrap();
    self.allocator.dealloc(token).unwrap();
    self.buffer.removes(offset, size);
    self.used_count -= size;
  }

  fn allocate_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32> {
    let offset = self.allocate_range_impl(v.len() as u32, relocation_handler);

    if let Some(offset) = offset {
      self.buffer.set_values(offset, v)?;
    }

    offset
  }

  fn allocate_range(
    &mut self,
    count: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32>
  where
    Self: LinearStorageDirectAccess,
  {
    self.allocate_range_impl(count, relocation_handler)
  }
}

pub type StorageBufferRangeAllocatePool<T> = RangeAllocatePool<StorageBufferReadOnlyDataView<[T]>>;
pub type RangeAllocatePool<T> = GPURangeAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;

pub fn create_storage_buffer_range_allocate_pool<T: Std430>(
  gpu: &GPU,
  init_size: u32,
  max_size: u32,
) -> StorageBufferRangeAllocatePool<T> {
  let buffer = StorageBufferReadOnlyDataView::<[T]>::create_by(
    &gpu.device,
    StorageBufferInit::Zeroed(NonZeroU64::new(init_size as u64).unwrap()),
  );

  let buffer = create_growable_buffer(gpu, buffer, max_size);
  GPURangeAllocateMaintainer::new(gpu, buffer)
}
