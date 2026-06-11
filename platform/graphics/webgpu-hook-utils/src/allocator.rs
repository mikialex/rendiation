use std::hash::Hash;
use std::ops::Range;

use database::RawEntityHandle;

use crate::*;

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GrowableRangeAllocator<K: Copy + Eq + Hash> {
  max_item_count: u32,
  current_count: u32,
  used_count: u32,
  // user_handle => (size, offset, handle)
  ranges: FastHashMap<K, (u32, u32, AllocationHandle)>,
  // todo, try other allocator that support relocate and shrink??
  allocator: xalloc::SysTlsf<u32>,
  label: String,
}

type Offset = u32;
type Size = u32;

#[derive(Debug)]
pub struct DataMoveMent {
  pub old_offset: u32,
  pub new_offset: u32,
  pub count: u32,
}

#[derive(Debug)]
pub struct BatchAllocateResult<K: Copy + Eq + Hash> {
  pub removed: FastHashSet<K>,
  /// failed_to_allocate may contain previous successful allocated handle
  pub failed_to_allocate: FastHashSet<K>,
  /// only contains previous allocated handle
  pub data_movements: FastHashMap<K, DataMoveMent>,
  /// only contains new allocated handle
  pub new_data_to_write: FastHashMap<K, (Offset, Size)>,
  pub resize_to: Option<u32>,
}

impl<K: Copy + Eq + Hash> BatchAllocateResult<K> {
  /// these three set should be exclusive
  pub fn change_count(&self) -> usize {
    self.removed.len() + self.failed_to_allocate.len() + self.data_movements.len()
  }
}

#[derive(Clone)]
pub struct BatchAllocateResultShared<K: Copy + Eq + Hash>(pub Arc<BatchAllocateResult<K>>, pub u32); // (_, u32_per_item)

impl<K: Copy + Eq + Hash> BatchAllocateResultShared<K> {
  pub fn has_data_movements(&self) -> bool {
    !self.0.data_movements.is_empty()
  }

  pub fn iter_data_movements(&self) -> impl Iterator<Item = BufferRelocate> + '_ {
    let u32_per_item = self.1 as u64;
    self.0.data_movements.values().map(move |v| BufferRelocate {
      self_offset: v.old_offset as u64 * u32_per_item * 4,
      target_offset: v.new_offset as u64 * u32_per_item * 4,
      count: v.count as u64 * u32_per_item * 4,
    })
  }

  pub fn access_new_change(&self, k: K) -> Option<[u32; 2]> {
    let u32_per_item = self.1;
    if let Some(v) = self.0.new_data_to_write.get(&k) {
      return Some([v.0 * u32_per_item, v.1 * u32_per_item]);
    }

    if let Some(v) = self.0.data_movements.get(&k) {
      return Some([v.new_offset * u32_per_item, v.count * u32_per_item]);
    }

    if self.0.failed_to_allocate.contains(&k) {
      return Some([DEVICE_RANGE_ALLOCATE_FAIL_MARKER, 0]);
    }

    None
  }
}

pub const DEVICE_RANGE_ALLOCATE_FAIL_MARKER: u32 = u32::MAX;

impl DataChanges for BatchAllocateResultShared<RawEntityHandle> {
  type Key = RawEntityHandle;
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

    // note, return count 0 for failed_to_allocate case is important
    let failed = self
      .0
      .failed_to_allocate
      .iter()
      .map(|k| (*k, [DEVICE_RANGE_ALLOCATE_FAIL_MARKER, 0]));

    movements.chain(new).chain(failed)
  }
}

impl<K: Copy + Eq + Hash> BatchAllocateResult<K> {
  fn notify_failed_to_allocate(&mut self, handle: K) {
    self.failed_to_allocate.insert(handle);
    self.data_movements.remove(&handle);
  }
  fn notify_data_move(&mut self, handle: K, movement: DataMoveMent) {
    self.failed_to_allocate.remove(&handle);
    if let Some(previous_movement) = self.data_movements.remove(&handle) {
      let movement = DataMoveMent {
        old_offset: previous_movement.old_offset,
        new_offset: movement.new_offset,
        count: movement.count,
      };
      self.data_movements.insert(handle, movement);
    } else {
      self.data_movements.insert(handle, movement);
    }
  }
}

impl<K: Copy + Eq + Hash> GrowableRangeAllocator<K> {
  pub fn new(label: &str, max_item_count: u32, init_count: u32) -> Self {
    assert!(init_count <= max_item_count);
    Self {
      max_item_count,
      current_count: init_count,
      used_count: 0,
      ranges: FastHashMap::with_capacity_and_hasher(init_count as usize, Default::default()),
      allocator: xalloc::SysTlsf::new(init_count),
      label: label.to_string(),
    }
  }

  /// Query a region by key. Returns (size, offset) if allocated.
  pub fn get_region(&self, key: K) -> Option<(u32, u32)> {
    self
      .ranges
      .get(&key)
      .map(|&(size, offset, _)| (size, offset))
  }

  pub fn update(
    &mut self,
    change_or_removed_keys: impl Iterator<Item = K>,
    new: impl IntoIterator<Item = (K, Size)> + Clone,
  ) -> BatchAllocateResult<K> {
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

    let new_size_requirement = new.clone().into_iter().map(|v| v.1).sum::<u32>();
    let new_init_count = new.clone().into_iter().count(); // we should merge the loop with the size_requirement
    let new_data_to_write =
      FastHashMap::with_capacity_and_hasher(new_init_count, Default::default());

    let new_init_for_move = if new_size_requirement > current_remain_capacity {
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
      FastHashMap::with_capacity_and_hasher(new_init_count, Default::default());

    if new_size_requirement > current_remain_capacity {
      let new_size = self.used_count + new_size_requirement;
      //  try to avoid fragmentation caused possible relocate
      let new_size = (new_size as f32 * 1.1) as u32;
      let new_size = new_size.min(self.max_item_count);

      // if we have reached the limit before, do nothing
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
      // the failed allocated key may also fail to allocated before
      if let Some((count, _, _)) = self.ranges.remove(k) {
        self.used_count -= count;
      }
    }

    result
  }

  fn relocate(
    &mut self,
    new_size: u32,
    results: &mut BatchAllocateResult<K>,
    new_inserted: &mut FastHashMap<K, (Size, Offset, AllocationHandle)>,
  ) {
    assert!(new_size > self.current_count);
    println!(
      "range allocator {} try grow from {} to {}, max {}",
      self.label, self.current_count, new_size, self.max_item_count
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

pub struct RangeAllocateBufferCollector<K: Copy + Eq + Hash> {
  small_buffer_writes: Vec<u8>,
  ///  handle -> small_buffer_writes offset
  small_buffer_mapping: FastHashMap<K, (usize, usize)>,
  large_buffer_writes: FastHashMap<K, (Arc<Vec<u8>>, Option<Range<usize>>)>,
}

impl<K: Copy + Eq + Hash> Default for RangeAllocateBufferCollector<K> {
  fn default() -> Self {
    Self {
      small_buffer_writes: Vec::new(),
      small_buffer_mapping: FastHashMap::default(),
      large_buffer_writes: FastHashMap::default(),
    }
  }
}

pub const SMALL_BUFFER_THRESHOLD_BYTE_COUNT: usize = 1024 * 5;

impl<K: Copy + Eq + Hash> RangeAllocateBufferCollector<K> {
  pub fn with_capacity(
    small_buffer_byte_writes: usize,
    small_buffer_count: usize,
    large_buffer_count: usize,
  ) -> Self {
    RangeAllocateBufferCollector {
      small_buffer_writes: Vec::with_capacity(small_buffer_byte_writes),
      small_buffer_mapping: FastHashMap::with_capacity_and_hasher(
        small_buffer_count,
        Default::default(),
      ),
      large_buffer_writes: FastHashMap::with_capacity_and_hasher(
        large_buffer_count,
        Default::default(),
      ),
    }
  }

  pub fn collect_shared(
    &mut self,
    handle: K,
    (buffer, range): (&Arc<Vec<u8>>, Option<Range<usize>>),
  ) {
    let buffer_slice = if let Some(range) = range.clone() {
      buffer.get(range).unwrap()
    } else {
      buffer.as_slice()
    };

    if buffer_slice.len() <= SMALL_BUFFER_THRESHOLD_BYTE_COUNT {
      self.collect_small(handle, buffer_slice);
    } else {
      self
        .large_buffer_writes
        .insert(handle, (buffer.clone(), range));
    }
  }
  pub fn collect_direct(&mut self, handle: K, bytes: &[u8]) {
    if bytes.len() <= SMALL_BUFFER_THRESHOLD_BYTE_COUNT {
      self.collect_small(handle, bytes);
    } else {
      self
        .large_buffer_writes
        .insert(handle, (Arc::new(bytes.to_vec()), None));
    }
  }

  fn collect_small(&mut self, handle: K, bytes: &[u8]) {
    assert_eq!(bytes.len() % 4, 0);
    let offset = self.small_buffer_writes.len();
    self.small_buffer_writes.extend_from_slice(bytes);
    self
      .small_buffer_mapping
      .insert(handle, (offset / 4, bytes.len() / 4));
  }

  pub fn prepare(
    self,
    allocation_changes: &BatchAllocateResult<K>,
    alloc_unit_item_byte_size: u32,
  ) -> RangeAllocateBufferPrepared<K> {
    let mut offset_size = Vec::with_capacity(self.small_buffer_mapping.len() * 3);

    for (k, (offset, size)) in self.small_buffer_mapping {
      // allocation may fail
      if let Some((write_offset, _)) = allocation_changes.new_data_to_write.get(&k) {
        offset_size.push(offset as u32);
        offset_size.push(size as u32);

        assert_eq!(write_offset * alloc_unit_item_byte_size % 4, 0);
        let write_offset = write_offset * alloc_unit_item_byte_size / 4;
        offset_size.push(write_offset);
      }
    }

    let small_buffer_writes = SparseBufferWritesSource {
      data_to_write: self.small_buffer_writes,
      offset_size,
    };

    RangeAllocateBufferPrepared {
      small_buffer_writes,
      large_buffer_writes: self.large_buffer_writes,
    }
  }
}

pub struct RangeAllocateBufferPrepared<K: Copy + Eq + Hash> {
  small_buffer_writes: SparseBufferWritesSource,
  large_buffer_writes: FastHashMap<K, (Arc<Vec<u8>>, Option<Range<usize>>)>,
}

pub struct RangeAllocateBufferUpdates<K: Copy + Eq + Hash> {
  pub buffers_to_write: RangeAllocateBufferPrepared<K>,
  pub allocation_changes: BatchAllocateResultShared<K>,
}

impl<K: Copy + Eq + Hash> RangeAllocateBufferUpdates<K> {
  pub fn write(&self, gpu: &GPU, encoder: &mut GPUCommandEncoder, target: &dyn AbstractBuffer) {
    if self.allocation_changes.has_data_movements() {
      let mut iter = self.allocation_changes.iter_data_movements();
      // we must use a standalone encoder, because the below code do queue write
      // todo, consider impl encoder write buffer to avoid this mental overhead
      let mut encoder = gpu.create_encoder();
      target.batch_self_relocate(&mut iter, &mut encoder, &gpu.device);
      gpu.submit_encoder(encoder);
    }

    let item_byte_size = self.allocation_changes.1 * 4;

    self
      .buffers_to_write
      .small_buffer_writes
      .write_abstract(gpu, encoder, target);

    for (k, (buffer, range)) in &self.buffers_to_write.large_buffer_writes {
      if let Some((write_offset, size)) = self.allocation_changes.0.new_data_to_write.get(k) {
        let buffer = if let Some(range) = range {
          &buffer[range.clone()]
        } else {
          buffer
        };
        assert_eq!(buffer.len(), (*size * item_byte_size) as usize);
        target.write(buffer, (write_offset * item_byte_size) as u64, &gpu.queue);
      }
    }
  }
}
