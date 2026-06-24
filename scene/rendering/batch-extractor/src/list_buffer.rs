use crate::*;

/// Host-side list of scene model entities for a single group (shader/material key).
/// The GPU data lives in the shared `SceneModelListPool`, not in an individual buffer.
pub struct PersistSceneModelListBuffer {
  pub host: Vec<RawEntityHandle>,
  pub group_key_hash: u64,
  pub mapping: FastHashMap<RawEntityHandle, usize>,
  pub updates: Option<PersistSceneModelListBufferMutation>,
}

pub struct PersistSceneModelListBufferMutation {
  pub mapping_change: FastHashMap<usize, u32>,
  pub new_len: usize,
}

impl PersistSceneModelListBufferMutation {
  pub fn new() -> Self {
    PersistSceneModelListBufferMutation {
      mapping_change: Default::default(),
      new_len: 0,
    }
  }

  pub fn into_sparse_update(self, base_offset: u32) -> Option<SparseBufferWritesSource> {
    let change_count = self.mapping_change.len();
    if change_count == 0 {
      return None;
    }
    let byte_change_capacity = change_count * 4;
    let mut updates = SparseBufferWritesSource::with_capacity(byte_change_capacity, change_count);

    for (idx, val) in self.mapping_change {
      updates.collect_write(bytes_of(&val), (base_offset + idx as u32) as u64 * 4);
    }

    updates.into()
  }
}

impl PersistSceneModelListBuffer {
  pub fn with_capacity(capacity: usize, group_key_hash: u64) -> Self {
    Self {
      group_key_hash,
      host: Vec::with_capacity(capacity),
      mapping: FastHashMap::with_capacity_and_hasher(capacity, Default::default()),
      updates: None,
    }
  }

  pub fn memory_usage(&self) -> usize {
    self.host.capacity() * std::mem::size_of::<RawEntityHandle>() + self.mapping.allocation_size()
  }

  pub fn insert(&mut self, sm_handle: RawEntityHandle) {
    let mutations = self
      .updates
      .get_or_insert_with(PersistSceneModelListBufferMutation::new);

    self.host.push(sm_handle);
    self.mapping.insert(sm_handle, self.host.len() - 1);
    mutations
      .mapping_change
      .insert(self.host.len() - 1, sm_handle.alloc_index());
    mutations.new_len = self.host.len();
  }

  pub fn remove(&mut self, sm_handle: RawEntityHandle) {
    let mutations = self
      .updates
      .get_or_insert_with(PersistSceneModelListBufferMutation::new);

    let idx = self.mapping.remove(&sm_handle).unwrap();
    let old_last_idx = self.host.len() - 1;
    if idx != old_last_idx {
      // The removed element was not the last; swap the tail into its place.
      let tail_item = self.host.last().cloned().unwrap();
      self.host.swap_remove(idx);
      self.mapping.insert(tail_item, idx);

      mutations.mapping_change.remove(&old_last_idx);
      mutations
        .mapping_change
        .insert(idx, tail_item.alloc_index());
    } else {
      // Removing the last element — just pop, no swap needed.
      self.host.pop();
      mutations.mapping_change.remove(&idx);
    }

    mutations.new_len = self.host.len();
  }

  pub fn representative(&self) -> Option<EntityHandle<SceneModelEntity>> {
    self
      .host
      .first()
      .map(|&raw| unsafe { EntityHandle::from_raw(raw) })
  }
}
