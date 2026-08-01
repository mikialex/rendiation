use std::hash::Hash;

use fast_hash_collection::*;

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GrowableRangeAllocator<K> {
  max_item_count: u32,
  current_count: u32,
  used_count: u32,
  // user_handle => (size, offset, handle)
  ranges: FastHashMap<K, (u32, u32, AllocationHandle)>,
  // todo, try other allocator that support relocate and shrink??
  allocator: xalloc::SysTlsf<u32>,
  alignment_require: u32,
  label: String,
}

impl<K: Clone + Eq + Hash> GrowableRangeAllocator<K> {
  pub fn new(label: &str, max_item_count: u32, init_count: u32, alignment_require: u32) -> Self {
    assert!(init_count <= max_item_count);
    Self {
      max_item_count,
      alignment_require,
      current_count: init_count,
      used_count: 0,
      ranges: FastHashMap::with_capacity_and_hasher(init_count as usize, Default::default()),
      allocator: xalloc::SysTlsf::new(init_count),
      label: label.to_string(),
    }
  }

  /// Query a region by key. Returns (size, offset) if allocated.
  pub fn get_region(&self, key: &K) -> Option<(u32, u32)> {
    self
      .ranges
      .get(key)
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

    // the caller must put every already allocated key that appears in `new`
    // into change_or_removed_keys, otherwise the old range would never be
    // released and used_count would be over counted
    #[cfg(debug_assertions)]
    for (k, _) in new.clone() {
      assert!(
        !self.ranges.contains_key(&k) || removed.contains(&k),
        "new key must be released first via change_or_removed_keys"
      );
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
        if let Some((token, offset)) = self.allocator.alloc_aligned(count, self.alignment_require) {
          self.used_count += count;

          result.new_data_to_write.insert(k.clone(), (offset, count));
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
      if let Some((new_token, new_offset)) =
        self.allocator.alloc_aligned(*count, self.alignment_require)
      {
        results.notify_data_move(
          k.clone(),
          DataMoveMent {
            old_offset: *offset,
            new_offset,
            count: *count,
          },
        );

        *token = new_token;
        *offset = new_offset;
      } else {
        results.notify_failed_to_allocate(k.clone());
      }
    }
    for (k, (count, offset, token)) in new_inserted.iter_mut() {
      if let Some((new_token, new_offset)) =
        self.allocator.alloc_aligned(*count, self.alignment_require)
      {
        results
          .new_data_to_write
          .insert(k.clone(), (new_offset, *count));

        *token = new_token;
        *offset = new_offset;
      } else {
        results.notify_failed_to_allocate(k.clone());
      }
    }
  }
}

type Offset = u32;
type Size = u32;

#[derive(Debug)]
pub struct DataMoveMent {
  pub old_offset: u32,
  pub new_offset: u32,
  pub count: u32,
}

// these four set/map should be exclusive
#[derive(Debug)]
pub struct BatchAllocateResult<K> {
  pub removed: FastHashSet<K>,
  /// failed_to_allocate may contain previous successful allocated handle
  pub failed_to_allocate: FastHashSet<K>,
  /// only contains previous allocated handle
  pub data_movements: FastHashMap<K, DataMoveMent>,
  /// only contains new allocated handle
  pub new_data_to_write: FastHashMap<K, (Offset, Size)>,
  pub resize_to: Option<u32>,
}

impl<K: Clone + Eq + Hash> BatchAllocateResult<K> {
  fn notify_failed_to_allocate(&mut self, handle: K) {
    self.failed_to_allocate.insert(handle.clone());
    // the handle may be in the other three collections, keep them exclusive
    self.data_movements.remove(&handle);
    self.new_data_to_write.remove(&handle);
    self.removed.remove(&handle);
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

pub enum AllocateChangeType {
  FailedToAllocate,
  Allocated([u32; 2]),
}

impl<K> BatchAllocateResult<K> {
  pub fn change_count(&self) -> usize {
    self.removed.len()
      + self.failed_to_allocate.len()
      + self.data_movements.len()
      + self.new_data_to_write.len()
  }

  pub fn access_new_change(&self, k: K) -> Option<AllocateChangeType>
  where
    K: Eq + Hash,
  {
    if let Some(v) = self.new_data_to_write.get(&k) {
      return Some(AllocateChangeType::Allocated([v.0, v.1]));
    }

    if let Some(v) = self.data_movements.get(&k) {
      return Some(AllocateChangeType::Allocated([v.new_offset, v.count]));
    }

    if self.failed_to_allocate.contains(&k) {
      return Some(AllocateChangeType::FailedToAllocate);
    }

    None
  }

  pub fn iter_update_or_insert(&self) -> impl Iterator<Item = (K, AllocateChangeType)> + '_
  where
    K: Copy,
  {
    let movements = self
      .data_movements
      .iter()
      .map(move |(k, v)| (*k, AllocateChangeType::Allocated([v.new_offset, v.count])));
    let new = self
      .new_data_to_write
      .iter()
      .map(move |(k, v)| (*k, AllocateChangeType::Allocated([v.0, v.1])));

    // note, return count 0 for failed_to_allocate case is important
    let failed = self
      .failed_to_allocate
      .iter()
      .map(|k| (*k, AllocateChangeType::FailedToAllocate));

    movements.chain(new).chain(failed)
  }
}

#[cfg(test)]
mod test;
