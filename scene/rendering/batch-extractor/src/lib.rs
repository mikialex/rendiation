use std::sync::Arc;

use database::*;
use fast_hash_collection::*;
use interning::InternedId;
use parking_lot::RwLock;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod list_buffer;
pub use list_buffer::*;

mod list_pool;
pub use list_pool::*;

mod default_key_logic;
pub use default_key_logic::*;

mod extractor;
pub use extractor::{
  ExtractorUpdate, IncrementalDeviceSceneBatchExtractor, IncrementalDeviceSceneBatchExtractorShared,
};

pub fn use_incremental_device_scene_batch_extractor<K: CKey>(
  cx: &mut QueryGPUHookCx,
  sm_group_key_with_scene_id: UseResult<BoxedDynDualQuery<RawEntityHandle, (K, RawEntityHandle)>>,
) -> Option<LockReadGuardHolder<IncrementalDeviceSceneBatchExtractor<K>>> {
  let (cx, (allocator, extractor)) = cx.use_gpu_init(|gpu, allocator| {
    let pool = SceneModelListPool::new(allocator, gpu, 1024);
    let allocator = pool.allocator_shared();
    let extractor = Arc::new(RwLock::new(IncrementalDeviceSceneBatchExtractor::new(pool)));
    (allocator, extractor)
  });

  let allocator = allocator.clone();
  let extractor = extractor.clone();

  cx.if_inspect(|inspector| {
    let bytes = extractor.read().memory_usage();
    inspector.label_memory_usage("indirect group key", bytes);
  });

  let extractor_ = extractor.clone();
  let gpu_updates = sm_group_key_with_scene_id
    .map_spawn_stage_in_thread_dual_query(cx, move |v| {
      let change = v.delta();
      let update = extractor_.write().prepare_updates(change, &allocator);
      Arc::new(update)
    })
    .use_assure_result(cx);

  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    extractor
      .write()
      .do_updates(&gpu_updates.expect_resolve_stage().0, cx.gpu, encoder);

    Some(extractor.make_read_holder())
  } else {
    None
  }
}
