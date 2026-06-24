use std::sync::Arc;

use parking_lot::RwLock;

use crate::*;

/// Shared pool of scene model entity indices with per-group contiguous regions
/// managed by a range allocator. Allocator updates happen in the spawn stage;
/// GPU buffer writes (resize, relocation, sparse write) happen in the render stage.
pub struct SceneModelListPool {
  /// The pool GPU buffer
  pool_buffer: ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[u32]>>,
  /// Range allocator — group_hash(u64) → pool region
  pub allocator: Arc<RwLock<GrowableRangeAllocator<u64>>>,
  gpu: GPU,
}

/// Result of spawn-stage allocator update. Carried over to the render stage.
pub struct PoolAllocationUpdate {
  /// The raw allocator result (new_data_to_write, data_movements, resize_to, removed)
  pub allocation_result: BatchAllocateResult<u64>,
  pub move_writes: Vec<(u64, Vec<u8>)>,
  /// Pre-built sparse writes for entity data (positions already include pool offset)
  pub sparse_writes: SparseBufferWritesSource,
}

impl SceneModelListPool {
  pub fn new(alloc: &dyn AbstractStorageAllocator, gpu: &GPU, init_capacity: u32) -> Self {
    let pool_buffer = alloc
      .allocate_readonly(init_capacity as u64 * 4, &gpu.device, "scene_model_id_pool")
      .with_direct_resize(gpu);

    let limits = &gpu.info.supported_limits;
    let bind_alignment_requirement_in_u32 = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    Self {
      pool_buffer,
      allocator: Arc::new(RwLock::new(GrowableRangeAllocator::new(
        "scene_model_id_pool allocator",
        u32::MAX,
        init_capacity,
        bind_alignment_requirement_in_u32,
      ))),
      gpu: gpu.clone(),
    }
  }

  pub fn update_pool_size(&mut self, new_size: u32) {
    let success = self.pool_buffer.resize(new_size);
    assert!(success);
  }

  pub fn pool_buffer(&self) -> &AbstractReadonlyStorageBuffer<[u32]> {
    &self.pool_buffer.gpu
  }

  pub fn pool_buffer_readonly(&self) -> StorageBufferReadonlyDataView<[u32]> {
    let view = self.pool_buffer.gpu.get_gpu_buffer_view().unwrap();
    StorageBufferReadonlyDataView::try_from_raw(view).unwrap()
  }

  pub fn gpu(&self) -> &GPU {
    &self.gpu
  }

  pub fn allocator_shared(&self) -> Arc<RwLock<GrowableRangeAllocator<u64>>> {
    self.allocator.clone()
  }

  pub fn prepare_pool_update(
    allocator: &Arc<RwLock<GrowableRangeAllocator<u64>>>,
    removed_groups: &[u64],
    changed_groups: &[(u64, Vec<u8>, u32)],
    entity_writes: Vec<(u64, u32, u32)>, // (group_hash, local_pos, entity_alloc_index)
  ) -> PoolAllocationUpdate {
    let mut alloc = allocator.write();

    // todo, the allocator's update can be improved:
    // if the changed groups overlaps remove_or_changed, it will not reflected as the data movement.
    let allocation_result = alloc.update(
      removed_groups
        .iter()
        .copied()
        .chain(changed_groups.iter().map(|v| v.0)),
      changed_groups
        .iter()
        .map(|(k, _data, new_len)| (*k, *new_len)),
    );

    let mut writes = SparseBufferWritesSource::default();
    for (group_hash, local_pos, value) in &entity_writes {
      let offset = alloc.get_region(*group_hash).unwrap().1;
      writes.collect_write(bytes_of(value), (offset + local_pos) as u64 * 4);
    }

    let mut move_writes = Vec::with_capacity(changed_groups.len());
    for group in changed_groups {
      let offset = alloc.get_region(group.0).unwrap().1;
      move_writes.push((offset as u64 * 4, group.1.clone()))
    }

    PoolAllocationUpdate {
      allocation_result,
      move_writes,
      sparse_writes: writes,
    }
  }

  pub fn apply_pool_update(
    &mut self,
    update: &PoolAllocationUpdate,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    // the pool buffer may have been resized in spawn stage
    if !update.allocation_result.data_movements.is_empty() {
      let mut encoder = gpu.create_encoder();
      let mut relocations =
        update
          .allocation_result
          .data_movements
          .values()
          .map(|v| BufferRelocate {
            self_offset: v.old_offset as u64 * 4,
            target_offset: v.new_offset as u64 * 4,
            count: v.count as u64 * 4,
          });
      self
        .pool_buffer
        .gpu
        .batch_self_relocate(&mut relocations, &mut encoder, &gpu.device);
      gpu.submit_encoder(encoder);
    }

    let buffer = self.pool_buffer.gpu.get_gpu_buffer_view().unwrap();
    let buffer = buffer.buffer.gpu();
    for (offset, data) in &update.move_writes {
      gpu.queue.write_buffer(buffer, *offset, data);
    }

    update
      .sparse_writes
      .write_abstract(gpu, encoder, self.pool_buffer());
  }
}
