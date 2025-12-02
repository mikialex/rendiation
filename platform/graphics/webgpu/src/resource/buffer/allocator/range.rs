use crate::*;

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GPURangeAllocateMaintainer<T> {
  max_item_count: u32,
  used_count: u32,
  // todo, remove this if we can get offset from handle in allocator
  // offset => (size, handle)
  ranges: FastHashMap<u32, (u32, AllocationHandle)>,
  // todo, try other allocator that support relocate and shrink??
  allocator: xalloc::SysTlsf<u32>,
  buffer: T,
  gpu: GPU,
}

impl<T> GPURangeAllocateMaintainer<T>
where
  T: ResizableLinearStorage + GPULinearStorage + LinearStorageDirectAccess,
{
  pub fn new(gpu: &GPU, buffer: T, max_item_count: u32) -> Self {
    let current_size = buffer.max_size();
    assert!(current_size <= max_item_count);
    Self {
      max_item_count,
      allocator: xalloc::SysTlsf::new(current_size),
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
    // do this before the resize, make sure we hold the old buffer reference here.
    let src = self.buffer.abstract_gpu().get_gpu_buffer_view().unwrap();
    if !self.buffer.resize(new_size) {
      return false;
    }
    assert!(self.buffer.max_size() >= new_size);
    let target = self.buffer.abstract_gpu().get_gpu_buffer_view().unwrap();

    // move the data
    let mut encoder = self.gpu.device.create_encoder();
    let mut new_allocator = xalloc::SysTlsf::new(new_size);

    let item_byte_width = std::mem::size_of::<T::Item>() as u32;
    let mut new_ranges = FastHashMap::default();
    self.ranges.iter_mut().for_each(|(old_offset, (size, _))| {
      let (new_token, new_offset) = new_allocator
        .alloc(*size)
        .expect("allocation after grow should success");

      // we currently do not use the abstract buffer's copy buffer to buffer
      // because the abstract buffer's resize maybe has lazy implementation, that will not guaranteed
      // to create new buffer and may cause copy buffer to buffer fail due to same buffer reference
      //
      // using temp buffer to work around is not an option, because the copy range may overlap

      // todo: abstract buffer's copy buffer to buffer
      encoder.copy_buffer_to_buffer(
        src.resource.gpu(),
        src.desc.offset + *old_offset as u64 * item_byte_width as u64,
        target.resource.gpu(),
        target.desc.offset + new_offset as u64 * item_byte_width as u64,
        *size as u64 * item_byte_width as u64,
      );

      new_ranges.insert(new_offset, (*size, new_token));
      relocation_handler(RelocationMessage {
        previous_offset: *old_offset,
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
      } else {
        let current_size = self.buffer.max_size();
        let next_allocate = (current_size * 2).max(count).min(self.max_item_count);
        assert!(next_allocate > current_size);
        log::info!(
          "range allocator try grow from {current_size} to {next_allocate}, max {}",
          self.max_item_count
        );
        if !self.relocate(next_allocate, relocation_handler) {
          return None;
        }
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

  fn gpu(&self) -> &Self::GPUType {
    self.buffer.gpu()
  }

  fn abstract_gpu(&mut self) -> &mut dyn AbstractBuffer {
    self.buffer.abstract_gpu()
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
  ) -> Option<u32> {
    self.allocate_range_impl(count, relocation_handler)
  }
}

pub type StorageBufferRangeAllocatePool<T> = RangeAllocatePool<AbstractReadonlyStorageBuffer<[T]>>;
pub type RangeAllocatePool<T> = GPURangeAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;

pub fn create_storage_buffer_range_allocate_pool<T: Std430 + ShaderSizedValueNodeType>(
  gpu: &GPU,
  allocator: &dyn AbstractStorageAllocator,
  label: &str,
  init_item_count: u32,
  max_item_count: u32,
) -> StorageBufferRangeAllocatePool<T> {
  assert!(max_item_count >= init_item_count);

  let byte_size = init_item_count as usize * std::mem::size_of::<T>();
  let buffer = allocator.allocate_readonly(byte_size as u64, &gpu.device, Some(label));

  let buffer = create_growable_buffer(gpu, buffer, max_item_count);
  GPURangeAllocateMaintainer::new(gpu, buffer, max_item_count)
}
