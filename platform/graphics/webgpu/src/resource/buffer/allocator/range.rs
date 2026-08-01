use growable_range_allocator::*;

use crate::*;

pub struct GPURangeAllocateMaintainer<T> {
  // the key of the inner allocator is a monotonically increasing id,
  // the caller uses the offset as the handle, so we keep a reverse
  // offset => id map for deallocate and relocation
  allocator: GrowableRangeAllocator<u32>,
  next_id: u32,
  offset_to_id: FastHashMap<u32, u32>,
  buffer: T,
}

impl<T> GPURangeAllocateMaintainer<T>
where
  T: RelocationResizableLinearStorage + GPULinearStorage + LinearStorageDirectAccess,
{
  pub fn new(buffer: T, max_item_count: u32, label: &str) -> Self {
    let current_size = buffer.max_size();
    assert!(current_size <= max_item_count);
    Self {
      allocator: GrowableRangeAllocator::new(label, max_item_count, current_size, 1),
      next_id: 0,
      offset_to_id: Default::default(),
      buffer,
    }
  }

  fn apply_resize_and_relocations(
    &mut self,
    result: &BatchAllocateResult<u32>,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) {
    if let Some(new_size) = result.resize_to {
      let item_byte_width = std::mem::size_of::<T::Item>() as u64;
      let resize_success = self.buffer.resize_with_relocations(
        new_size,
        Some(
          &mut result.data_movements.values().map(move |m| BufferRelocate {
            self_offset: m.old_offset as u64 * item_byte_width,
            target_offset: m.new_offset as u64 * item_byte_width,
            count: m.count as u64 * item_byte_width,
          }) as _,
        ),
      );
      assert!(resize_success);
      for m in result.data_movements.values() {
        let id = self.offset_to_id.remove(&m.old_offset).unwrap();
        self.offset_to_id.insert(m.new_offset, id);
        relocation_handler(RelocationMessage {
          previous_offset: m.old_offset,
          new_offset: m.new_offset,
        })
      }
    } else {
      assert!(result.data_movements.is_empty());
    }
  }

  fn allocate_range_impl(
    &mut self,
    count: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32> {
    assert!(count > 0);
    let id = self.next_id;
    self.next_id += 1;
    let result = self.allocator.update([].into_iter(), [(id, count)]);
    if result.failed_to_allocate.contains(&id) {
      return None;
    }
    self.apply_resize_and_relocations(&result, relocation_handler);
    let offset = result.new_data_to_write.get(&id).unwrap().0;
    self.offset_to_id.insert(offset, id);
    Some(offset)
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
    self.allocator.current_used()
  }
}

impl<T> RangeAllocatorStorage for GPURangeAllocateMaintainer<T>
where
  T: RelocationResizableLinearStorage + LinearStorageDirectAccess + GPULinearStorage,
{
  fn deallocate(&mut self, offset: u32) {
    let id = self.offset_to_id.remove(&offset).unwrap();
    let (size, _) = self.allocator.get_region(&id).unwrap();
    self.allocator.update([id].into_iter(), []);
    self.buffer.removes(offset, size);
  }

  fn allocate_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32> {
    let offset = self.allocate_range_impl(v.len() as u32, relocation_handler)?;
    self.buffer.set_values(offset, v)?;
    Some(offset)
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
  let buffer = allocator.allocate_readonly(byte_size as u64, &gpu.device, label);

  let buffer = create_growable_buffer(gpu, buffer, max_item_count);
  GPURangeAllocateMaintainer::new(buffer, max_item_count, label)
}
