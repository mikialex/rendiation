use std::hash::Hash;
use std::ops::Range;

pub use growable_range_allocator::*;

use crate::*;

#[derive(Clone)]
pub struct BatchAllocateResultShared<K> {
  // wrap in arc to make it cheap to clone
  internal: Arc<BatchAllocateResult<K>>,
  // deliberately not use byte per item because gpu as minimal 4 byte alignment in copy cmd.
  u32_per_item: u32,
}

impl<K> BatchAllocateResultShared<K> {
  pub fn new(internal: BatchAllocateResult<K>, u32_per_item: u32) -> Self {
    Self {
      internal: Arc::new(internal),
      u32_per_item,
    }
  }
}

impl<K> BatchAllocateResultShared<K> {
  pub fn change_count(&self) -> usize {
    self.internal.change_count()
  }

  pub fn apply_resize(&self, gpu_buffer: &mut impl RelocationResizableLinearStorage) {
    if let Some(new_size) = self.internal.resize_to {
      // here we do(request) resize at spawn stage to avoid resize again and again(with use of combined buffer)
      let resize_success = gpu_buffer.resize_with_relocations(
        new_size,
        self.iter_data_movements().as_mut().map(|v| v as _),
      );
      assert!(resize_success);
    } else {
      assert!(self.iter_data_movements().is_none());
    }
  }

  // explicitly return Option to avoid encoder create cost when there is no movement at all
  fn iter_data_movements(&self) -> Option<impl Iterator<Item = BufferRelocate> + '_> {
    if !self.internal.data_movements.is_empty() {
      let u32_per_item = self.u32_per_item as u64;
      self
        .internal
        .data_movements
        .values()
        .map(move |v| BufferRelocate {
          self_offset: v.old_offset as u64 * u32_per_item * 4,
          target_offset: v.new_offset as u64 * u32_per_item * 4,
          count: v.count as u64 * u32_per_item * 4,
        })
        .into()
    } else {
      None
    }
  }

  pub fn access_new_change(&self, k: K) -> Option<[u32; 2]>
  where
    K: Eq + Hash,
  {
    self.internal.access_new_change(k).map(convert_failed_alloc)
  }
}

pub const DEVICE_RANGE_ALLOCATE_FAIL_MARKER: u32 = u32::MAX;

fn convert_failed_alloc(change: AllocateChangeType) -> [u32; 2] {
  match change {
    AllocateChangeType::FailedToAllocate => [DEVICE_RANGE_ALLOCATE_FAIL_MARKER, 0],
    AllocateChangeType::Allocated(r) => r,
  }
}

impl<K: CKey + Copy> DataChanges for BatchAllocateResultShared<K> {
  type Key = K;
  type Value = [u32; 2];

  fn has_change(&self) -> bool {
    self.internal.change_count() != 0
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.internal.removed.iter().copied()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .internal
      .iter_update_or_insert()
      .map(|(k, v)| (k, convert_failed_alloc(v)))
  }
}

pub struct RangeAllocateBufferCollector<K> {
  small_buffer_writes: Vec<u8>,
  ///  handle -> small_buffer_writes offset
  small_buffer_mapping: FastHashMap<K, (usize, usize)>,
  large_buffer_writes: FastHashMap<K, (Arc<Vec<u8>>, Option<Range<usize>>)>,
}

impl<K> Default for RangeAllocateBufferCollector<K> {
  fn default() -> Self {
    Self {
      small_buffer_writes: Vec::new(),
      small_buffer_mapping: FastHashMap::default(),
      large_buffer_writes: FastHashMap::default(),
    }
  }
}

pub const SMALL_BUFFER_THRESHOLD_BYTE_COUNT: usize = 1024 * 5;

impl<K: Clone + Eq + Hash> RangeAllocateBufferCollector<K> {
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

pub struct RangeAllocateBufferPrepared<K> {
  small_buffer_writes: SparseBufferWritesSource,
  large_buffer_writes: FastHashMap<K, (Arc<Vec<u8>>, Option<Range<usize>>)>,
}

pub struct RangeAllocateBufferUpdates<K> {
  pub buffers_to_write: RangeAllocateBufferPrepared<K>,
  pub allocation_changes: BatchAllocateResultShared<K>,
}

impl<K: Clone + Eq + Hash> RangeAllocateBufferUpdates<K> {
  /// relocation is handled within resize, so the caller must call resize
  /// before this write
  pub fn write(&self, gpu: &GPU, encoder: &mut GPUCommandEncoder, target: &dyn AbstractBuffer) {
    let item_byte_size = self.allocation_changes.u32_per_item * 4;

    self
      .buffers_to_write
      .small_buffer_writes
      .write_abstract(gpu, encoder, target);

    for (k, (buffer, range)) in &self.buffers_to_write.large_buffer_writes {
      if let Some((write_offset, size)) = self.allocation_changes.internal.new_data_to_write.get(k)
      {
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
