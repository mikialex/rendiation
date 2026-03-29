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
