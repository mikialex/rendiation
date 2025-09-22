use std::ops::Range;

use database::RawEntityHandle;

use crate::*;

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GrowableRangeAllocator {
  max_item_count: u32,
  current_count: u32,
  used_count: u32,
  // user_handle => (size, offset, handle)
  ranges: FastHashMap<UserHandle, (u32, u32, AllocationHandle)>,
  // todo, try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
}

type UserHandle = RawEntityHandle;
type Offset = u32;
type Size = u32;

#[derive(Debug)]
pub struct DataMoveMent {
  pub old_offset: u32,
  pub new_offset: u32,
  pub count: u32,
}

#[derive(Debug)]
pub struct BatchAllocateResult {
  pub removed: FastHashSet<UserHandle>,
  /// failed_to_allocate may contains previous successful allocated handle
  pub failed_to_allocate: FastHashSet<UserHandle>,
  /// only contains previous allocated handle
  pub data_movements: FastHashMap<UserHandle, DataMoveMent>,
  /// only contains new allocated handle
  pub new_data_to_write: FastHashMap<UserHandle, (Offset, Size)>,
  pub resize_to: Option<u32>,
}

impl BatchAllocateResult {
  /// these three set should be exclusive
  pub fn change_count(&self) -> usize {
    self.removed.len() + self.failed_to_allocate.len() + self.data_movements.len()
  }
}

#[derive(Clone)]
pub struct BatchAllocateResultShared(pub Arc<BatchAllocateResult>, pub u32); // (_, u32_per_item)

impl DataChanges for BatchAllocateResultShared {
  type Key = UserHandle;
  type Value = [u32; 2];

  fn has_change(&self) -> bool {
    !self.0.failed_to_allocate.is_empty()
      || !self.0.data_movements.is_empty()
      || !self.0.new_data_to_write.is_empty()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.0.removed.iter().copied()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    let u32_per_item = self.1;
    let movements = self
      .0
      .data_movements
      .iter()
      .map(move |(k, v)| (*k, [v.new_offset * u32_per_item, v.count * u32_per_item]));
    let new = self
      .0
      .new_data_to_write
      .iter()
      .map(move |(k, v)| (*k, [v.0 * u32_per_item, v.1 * u32_per_item]));
    let failed = self
      .0
      .failed_to_allocate
      .iter()
      .map(|k| (*k, [u32::MAX, 0]));

    movements.chain(new).chain(failed)
  }
}

impl BatchAllocateResult {
  fn notify_failed_to_allocate(&mut self, handle: UserHandle) {
    self.failed_to_allocate.insert(handle);
    self.data_movements.remove(&handle);
  }
  fn notify_data_move(&mut self, handle: UserHandle, movement: DataMoveMent) {
    self.failed_to_allocate.remove(&handle);
    self.data_movements.insert(handle, movement);
  }
}

impl GrowableRangeAllocator {
  pub fn new(max_item_count: u32, init_count: u32) -> Self {
    assert!(init_count <= max_item_count);
    Self {
      max_item_count,
      current_count: init_count,
      used_count: 0,
      ranges: FastHashMap::with_capacity_and_hasher(init_count as usize, Default::default()),
      allocator: xalloc::SysTlsf::new(init_count),
    }
  }

  pub fn update(
    &mut self,
    change_or_removed_keys: impl Iterator<Item = UserHandle>,
    new: impl IntoIterator<Item = (UserHandle, Size)> + Clone,
  ) -> BatchAllocateResult {
    let mut removed = FastHashSet::with_capacity_and_hasher(
      change_or_removed_keys.size_hint().1.unwrap_or(0),
      Default::default(),
    );
    for k in change_or_removed_keys {
      if let Some((size, _offset, token)) = self.ranges.remove(&k) {
        self.allocator.dealloc(token).unwrap();
        self.used_count -= size;
        removed.insert(k);
      }
    }

    let current_remain_capacity = self.current_count - self.used_count;
    let size_requirement = new.clone().into_iter().map(|v| v.1).sum::<u32>();
    let new_init = new.clone().into_iter().count(); // we should merge the loop with the size_requirement

    let new_data_to_write = FastHashMap::with_capacity_and_hasher(new_init, Default::default());

    let new_init_for_move = if size_requirement > current_remain_capacity {
      self.ranges.len()
    } else {
      0
    };
    let data_movements =
      FastHashMap::with_capacity_and_hasher(new_init_for_move, Default::default());

    let mut result = BatchAllocateResult {
      failed_to_allocate: Default::default(),
      data_movements,
      new_data_to_write,
      resize_to: None,
      removed,
    };

    // use a separate hash map to avoid change the self.ranges
    let mut new_metadata_to_write =
      FastHashMap::with_capacity_and_hasher(new_init, Default::default());

    if size_requirement > current_remain_capacity {
      //  try to avoid fragmentation caused possible relocate
      let extra = self.current_count as f32 * 0.1;
      let new_size =
        (current_remain_capacity + size_requirement + extra as u32).min(self.max_item_count);
      if new_size != self.max_item_count {
        self.relocate(new_size, &mut result, &mut new_metadata_to_write);
      }
    }

    for (k, count) in new {
      assert!(count > 0);
      // even if we relocate before, we have to loop relocate here to prevent
      // allocated failed due to fragmentation
      loop {
        if let Some((token, offset)) = self.allocator.alloc(count) {
          self.used_count += count;

          result.new_data_to_write.insert(k, (offset, count));
          result.removed.remove(&k);
          new_metadata_to_write.insert(k, (count, offset, token));
          break;
        } else {
          let next_allocate = (self.current_count * 2).max(count).min(self.max_item_count);
          if next_allocate == self.current_count {
            result.notify_failed_to_allocate(k);
            println!("range allocator reach max allocation size",);
            break;
          }
          self.relocate(next_allocate, &mut result, &mut new_metadata_to_write);
          continue;
        }
      }
    }

    self.ranges.reserve(new_metadata_to_write.len());
    for (k, v) in new_metadata_to_write {
      self.ranges.insert(k, v);
    }

    for k in &result.failed_to_allocate {
      let (count, _, _) = self.ranges.remove(k).unwrap();
      self.used_count -= count;
    }

    result
  }

  fn relocate(
    &mut self,
    new_size: u32,
    results: &mut BatchAllocateResult,
    new_inserted: &mut FastHashMap<UserHandle, (Size, Offset, AllocationHandle)>,
  ) {
    println!(
      "range allocator try grow from {} to {}, max {}",
      self.current_count, new_size, self.max_item_count
    );
    self.current_count = new_size;
    results.resize_to = Some(new_size);
    self.allocator = xalloc::SysTlsf::new(new_size);
    for (k, (count, offset, token)) in self.ranges.iter_mut() {
      if let Some((new_token, new_offset)) = self.allocator.alloc(*count) {
        results.notify_data_move(
          *k,
          DataMoveMent {
            old_offset: *offset,
            new_offset,
            count: *count,
          },
        );

        *token = new_token;
        *offset = new_offset;
      } else {
        results.notify_failed_to_allocate(*k);
      }
    }
    for (k, (count, offset, token)) in new_inserted.iter_mut() {
      if let Some((new_token, new_offset)) = self.allocator.alloc(*count) {
        results.new_data_to_write.insert(*k, (new_offset, *count));

        *token = new_token;
        *offset = new_offset;
      } else {
        results.notify_failed_to_allocate(*k);
      }
    }
  }
}

pub struct RangeAllocateBufferUpdates {
  pub buffers_to_write: FastHashMap<RawEntityHandle, (Arc<Vec<u8>>, Option<Range<usize>>)>,
  pub allocation_changes: BatchAllocateResultShared,
  pub source_buffer: Option<GPUBufferResourceView>,
}

impl RangeAllocateBufferUpdates {
  pub fn write(&self, gpu: &GPU, gpu_buffer: &GPUBufferResourceView, item_byte_size: u32) {
    if let Some(source) = &self.source_buffer {
      let mut encoder = gpu.create_encoder();
      for (_, movement) in &self.allocation_changes.0.data_movements {
        encoder.copy_buffer_to_buffer(
          source.resource.gpu(),
          source.desc.offset + (movement.old_offset * item_byte_size) as u64,
          gpu_buffer.resource.gpu(),
          gpu_buffer.desc.offset + (movement.new_offset * item_byte_size) as u64,
          (movement.count * item_byte_size) as u64,
        );
      }
      gpu.queue.submit_encoder(encoder);
    }

    for (k, (write_offset, size)) in &self.allocation_changes.0.new_data_to_write {
      let (buffer, range) = self.buffers_to_write.get(k).unwrap();
      let buffer = if let Some(range) = range {
        &buffer[range.clone()]
      } else {
        buffer
      };
      assert_eq!(buffer.len(), (*size * item_byte_size) as usize);
      gpu.queue.write_buffer(
        gpu_buffer.resource.gpu(),
        (write_offset * item_byte_size) as u64 + gpu_buffer.desc.offset,
        buffer,
      );
    }
  }
}
