use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use database::*;
use fast_hash_collection::*;
use interning::InternedId;
use parking_lot::RwLock;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod list_buffer;
use list_buffer::*;

mod default_key_logic;
pub use default_key_logic::*;

mod extractor;
pub use extractor::{
  IncrementalDeviceSceneBatchExtractor, IncrementalDeviceSceneBatchExtractorShared,
};

pub fn use_incremental_device_scene_batch_extractor<K: CKey>(
  cx: &mut QueryGPUHookCx,
  sm_group_key_with_scene_id: UseResult<BoxedDynDualQuery<RawEntityHandle, (K, RawEntityHandle)>>,
) -> Option<IncrementalDeviceSceneBatchExtractorShared<K>> {
  let (cx, extractor) =
    cx.use_plain_state_default_cloned::<IncrementalDeviceSceneBatchExtractorShared<K>>();

  cx.if_inspect(|inspector| {
    let bytes = extractor.read().memory_usage();
    inspector.label_memory_usage("indirect group key", bytes);
  });

  let extractor_ = extractor.clone();
  let gpu_updates = sm_group_key_with_scene_id
    .map_spawn_stage_in_thread_dual_query(cx, move |v| {
      let change = v.delta();
      Arc::new(extractor_.write().prepare_updates(change))
    })
    .use_assure_result(cx);

  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    extractor.write().do_updates(
      &gpu_updates.expect_resolve_stage(),
      &cx.storage_allocator,
      cx.gpu,
      encoder,
    );

    Some(extractor)
  } else {
    None
  }
}
