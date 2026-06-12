use std::sync::Arc;

use parking_lot::RwLock;

use crate::*;

/// Shared pool of scene model entity indices with per-group contiguous regions
/// managed by a range allocator. Allocator updates happen in the spawn stage;
/// GPU buffer writes (resize, relocation, sparse write) happen in the render stage.
pub struct SceneModelListPool {
  /// The pool GPU buffer
  pool_buffer: AbstractReadonlyStorageBuffer<[u32]>,
  /// Range allocator — group_hash(u64) → pool region
  pub allocator: Arc<RwLock<GrowableRangeAllocator<u64>>>,
  gpu: GPU,
}

/// Result of spawn-stage allocator update. Carried over to the render stage.
pub struct PoolAllocationUpdate {
  /// The raw allocator result (new_data_to_write, data_movements, resize_to, removed)
  pub allocation_result: BatchAllocateResult<u64>,
  /// Pre-built sparse writes for entity data (positions already include pool offset)
  pub sparse_writes: SparseBufferWritesSource,
}

impl SceneModelListPool {
  pub fn new(alloc: &dyn AbstractStorageAllocator, gpu: &GPU, init_capacity: u32) -> Self {
    let pool_buffer = alloc.allocate_readonly(
      init_capacity as u64 * 4,
      &gpu.device,
      Some("scene_model_pool"),
    );

    let limits = &gpu.info.supported_limits;
    let bind_alignment_requirement_in_u32 = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    Self {
      pool_buffer,
      allocator: Arc::new(RwLock::new(GrowableRangeAllocator::new(
        "scene_model_pool",
        u32::MAX,
        init_capacity,
        bind_alignment_requirement_in_u32,
      ))),
      gpu: gpu.clone(),
    }
  }

  pub fn pool_buffer(&self) -> &AbstractReadonlyStorageBuffer<[u32]> {
    &self.pool_buffer
  }

  pub fn pool_buffer_readonly(&self) -> StorageBufferReadonlyDataView<[u32]> {
    let view = self.pool_buffer.get_gpu_buffer_view().unwrap();
    StorageBufferReadonlyDataView::try_from_raw(view).unwrap()
  }

  pub fn gpu(&self) -> &GPU {
    &self.gpu
  }

  /// Clone the allocator Arc for shared access during spawn stage.
  pub fn allocator_shared(&self) -> Arc<RwLock<GrowableRangeAllocator<u64>>> {
    self.allocator.clone()
  }

  /// Spawn-stage: process group allocation changes and build sparse writes.
  /// `changed_groups`: iterator of (group_hash, removed_old, new_size)
  ///   where `removed_old` is true if the group previously had an allocation that should be freed.
  pub fn prepare_pool_update(
    allocator: &Arc<RwLock<GrowableRangeAllocator<u64>>>,
    changed_groups: &[(u64, bool, u32)],
    entity_writes: Vec<(u64, u32, u32)>, // (group_hash, local_pos, entity_alloc_index)
  ) -> PoolAllocationUpdate {
    let mut alloc = allocator.write();
    let removed = changed_groups
      .iter()
      .filter(|(_, removed_old, _)| *removed_old)
      .map(|(hash, _, _)| *hash);
    let new_allocs: Vec<_> = changed_groups
      .iter()
      .map(|(hash, _, size)| (*hash, *size))
      .collect();

    let allocation_result = alloc.update(removed, new_allocs.clone());

    // Build sparse writes — apply pool offsets from allocator result.
    let mut writes = SparseBufferWritesSource::default();
    for (group_hash, local_pos, value) in &entity_writes {
      let offset = alloc.get_region(*group_hash).unwrap().1;
      writes.collect_write(bytes_of(value), (offset + local_pos) as u64 * 4);
    }

    PoolAllocationUpdate {
      allocation_result,
      sparse_writes: writes,
    }
  }

  /// Render-stage: apply the allocation update to the GPU.
  /// Handles pool buffer resize, GPU-side data relocation, and sparse writes.
  pub fn apply_pool_update(
    &mut self,
    update: &PoolAllocationUpdate,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    // 1. Resize pool buffer if allocator grew
    if let Some(new_capacity) = update.allocation_result.resize_to {
      let new_bytes = new_capacity as u64 * 4;
      if self.pool_buffer.byte_size() < new_bytes {
        self.pool_buffer.resize_gpu(encoder, &gpu.device, new_bytes);
      }
    }

    // 2. Handle data movements (GPU buffer self-relocate)
    if !update.allocation_result.data_movements.is_empty() {
      let mut reloc_encoder = gpu.create_encoder();
      let relocations: Vec<BufferRelocate> = update
        .allocation_result
        .data_movements
        .values()
        .map(|v| BufferRelocate {
          self_offset: v.old_offset as u64 * 4,
          target_offset: v.new_offset as u64 * 4,
          count: v.count as u64 * 4,
        })
        .collect();
      self.pool_buffer.batch_self_relocate(
        &mut relocations.into_iter(),
        &mut reloc_encoder,
        &gpu.device,
      );
      gpu.submit_encoder(reloc_encoder);
    }

    // 3. Write sparse entity data
    if !update.sparse_writes.is_empty() {
      update
        .sparse_writes
        .write_abstract(gpu, encoder, self.pool_buffer());
    }
  }
}
