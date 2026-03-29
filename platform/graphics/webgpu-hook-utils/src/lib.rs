#![feature(impl_trait_in_assoc_type)]
#![feature(iter_array_chunks)]
#![feature(cold_path)]

use std::sync::Arc;
use std::task::Waker;

use database::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
pub use query_hook::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod hook;
pub use hook::*;
mod use_result_ext;
pub use use_result_ext::*;
mod allocator;
pub use allocator::*;
mod multi_access;
pub use multi_access::*;
mod binding_array;
pub use binding_array::*;
mod sparse_buffer_writes;
pub use sparse_buffer_writes::*;
mod sparse_update_storage_buffer;
pub use sparse_update_storage_buffer::*;

pub type UniformArray<T, const N: usize> = UniformBufferDataView<Shader140Array<T, N>>;

pub fn use_db_device_foreign_key<S: ForeignKeySemantic>(
  cx: &mut QueryGPUHookCx,
) -> Option<AbstractReadonlyStorageBuffer<[u32]>> {
  let (cx, label) = cx.use_plain_state(|| format!("{} device foreign key", S::unique_name()));

  let (cx, device_mapping_buffer) = cx.use_storage_buffer::<u32>(label, 128, u32::MAX);

  cx.use_dual_query::<S>()
    .map_raw_handle_or_u32_max_changes()
    .update_storage_array(cx, device_mapping_buffer, 0);

  device_mapping_buffer.use_max_item_count_by_db_entity::<S::Entity>(cx);
  device_mapping_buffer.use_update(cx);

  cx.when_render(|| device_mapping_buffer.get_gpu_buffer())
}

pub fn use_range_allocated_device_buffers<T: Std430 + ShaderSizedValueNodeType>(
  cx: &mut QueryGPUHookCx,
  label: &str,
  init_item_count: u32,
  max_item_count: u32,
  data_source: UseResult<
    impl DataChanges<Key = RawEntityHandle, Value = ExternalRefPtr<Vec<u8>>> + 'static,
  >,
) -> (
  AbstractReadonlyStorageBuffer<[T]>,
  UseResult<Arc<RangeAllocateBufferUpdates>>,
) {
  let item_byte_size = std::mem::size_of::<T>() as u32;
  let (cx, gpu_target_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[T]>(
      (item_byte_size * init_item_count) as u64,
      &gpu.device,
      Some(label),
    );

    let buffer = buffer.with_direct_resize(gpu);

    Arc::new(RwLock::new(buffer))
  });

  cx.if_inspect(|inspector| {
    let buffer_size = gpu_target_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage(label, buffer_size);
  });

  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_item_count, init_item_count));

  let gpu_buffer = gpu_target_buffer.clone();

  let allocation_info = data_source.map_spawn_stage_in_thread_data_changes(cx, move |change| {
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    // todo, avoid resize
    let mut buffers_to_write = RangeAllocateBufferCollector::default();
    let mut sizes = Vec::new();

    for (k, buffer) in change.iter_update_or_insert() {
      let buffer = buffer.ptr.clone();

      let len = buffer.len() as u32;
      buffers_to_write.collect_direct(k, &buffer);
      sizes.push((k, len / item_byte_size));
    }

    let changes = allocator.write().update(removed_and_changed_keys, sizes);

    let buffers_to_write = buffers_to_write.prepare(&changes, item_byte_size);

    if let Some(new_size) = changes.resize_to {
      // here we do(request) resize at spawn stage to avoid resize again and again
      gpu_buffer.write().resize(new_size);
    }

    Arc::new(RangeAllocateBufferUpdates {
      buffers_to_write,
      allocation_changes: BatchAllocateResultShared(Arc::new(changes), item_byte_size / 4),
    })
  });

  let (allocation_info, allocation_info_) = allocation_info.fork();

  let allocation_info_ = allocation_info_.use_assure_result(cx);
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let mut gpu_buffer = gpu_target_buffer.write();
    let gpu_buffer = gpu_buffer.abstract_gpu();
    allocation_info_
      .expect_resolve_stage()
      .write(cx.gpu, encoder, gpu_buffer);
  }

  let buffer = gpu_target_buffer.read().gpu().clone();

  (buffer, allocation_info)
}
