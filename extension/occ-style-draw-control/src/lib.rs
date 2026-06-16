use std::sync::Arc;

use bytemuck::bytes_of;
use database::*;
use parking_lot::RwLock;
use rendiation_scene_batch_extractor::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;

mod gles;
pub use gles::*;

#[repr(C)]
#[derive(Serialize, Deserialize, Facet)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OccFlavorZLayer {
  BotOSD = 0,
  #[default]
  Default = 1,
  Top = 2,
  TopMost = 3,
  TopOSD = 4,
}

declare_component!(SceneModelOccStyleLayer, SceneModelEntity, OccFlavorZLayer);
declare_component!(SceneModelOccStylePriority, SceneModelEntity, u32);

pub fn register_occ_style_draw_control_data_model() {
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelOccStyleLayer>();
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelOccStylePriority>();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OccSceneModelGroupKey {
  pub internal: SceneModelGroupKey,
  pub layer: OccFlavorZLayer,
}

pub fn use_scene_model_occ_group_key(
  cx: &mut QueryGPUHookCx,
  foreign: GroupKeyForeignImpl,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, (OccSceneModelGroupKey, RawEntityHandle)>> {
  let internal = use_scene_model_group_key(cx, foreign);
  let layer = cx.use_dual_query::<SceneModelOccStyleLayer>();
  internal
    .dual_query_intersect(layer)
    .dual_query_map(|((k, s_id), layer)| {
      let key = OccSceneModelGroupKey { internal: k, layer };
      (key, s_id)
    })
    .dual_query_boxed()
}

pub fn use_occ_incremental_device_scene_batch_extractor(
  cx: &mut QueryGPUHookCx,
  sm_group_key_with_scene_id: UseResult<
    BoxedDynDualQuery<RawEntityHandle, (OccSceneModelGroupKey, RawEntityHandle)>,
  >,
) -> Option<Box<dyn SceneBatchBasicExtractAbility>> {
  let (cx, (allocator, extractor)) = cx.use_gpu_init(|gpu, allocator| {
    let pool = SceneModelListPool::new(allocator, gpu, 1024);
    let allocator = pool.allocator_shared();

    let extractor = Arc::new(RwLock::new(OccStyleOrderControlSceneBatchExtractor {
      internal: IncrementalDeviceSceneBatchExtractor::new(pool),
    }));
    (allocator, extractor)
  });

  let allocator = allocator.clone();
  let extractor = extractor.clone();

  cx.if_inspect(|inspector| {
    let bytes = extractor.read().internal.memory_usage();
    inspector.label_memory_usage("indirect group key", bytes);
  });

  let priority_changes = cx.use_dual_query::<SceneModelOccStylePriority>();

  let extractor_ = extractor.clone();
  let gpu_updates = sm_group_key_with_scene_id
    .join(priority_changes)
    .map_spawn_stage_in_thread(
      cx,
      |(c1, c2)| c1.has_delta_hint() || c2.has_delta_hint(),
      move |(c1, c2)| {
        Arc::new(
          extractor_
            .write()
            .prepare_updates(c1, c2.delta(), &allocator),
        )
      },
    )
    .use_assure_result(cx);

  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    extractor
      .write()
      .do_updates(&gpu_updates.expect_resolve_stage(), cx.gpu, encoder);

    Some(Box::new(extractor.make_read_holder()))
  } else {
    None
  }
}

pub struct OccStyleOrderControlSceneBatchExtractor {
  pub internal: IncrementalDeviceSceneBatchExtractor<OccSceneModelGroupKey>,
}

impl SceneBatchBasicExtractAbility for OccStyleOrderControlSceneBatchExtractor {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    _renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    let contents = self.internal.contents.get(&scene.into_raw());
    let Some(contents) = contents else {
      return SceneModelRenderBatch::Device(None);
    };

    let mut groups: Vec<_> = if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
      contents
        .iter()
        .filter(|(k, _)| k.internal.require_alpha_blend() == alpha_blend)
        .collect()
    } else {
      contents.iter().collect()
    };

    if groups.is_empty() {
      return SceneModelRenderBatch::Device(None);
    }

    groups.sort_by_key(|(k, _)| k.layer as u32);

    let mut impl_select_ids = Vec::with_capacity(groups.len());
    let mut capacity_ranges = Vec::with_capacity(groups.len());
    let mut real_lengths = Vec::with_capacity(groups.len());

    let alloc = self.internal.pool.allocator.read();
    for (_key, buffer) in &groups {
      impl_select_ids.push(buffer.representative().unwrap());
      real_lengths.push(buffer.host.len() as u32);
      let hash = buffer.group_key_hash;
      let (capacity, offset) = alloc.get_region(hash).unwrap();
      capacity_ranges.push(CapacityRange { capacity, offset });
    }
    drop(alloc);

    let sum_all_count_host: u32 = groups.iter().map(|(_, buf)| buf.host.len() as u32).sum();
    let gpu = self.internal.pool.gpu();
    let ranges_gpu = prepare_gpu_sub_list_ranges(&capacity_ranges, &real_lengths);
    let sub_list_ranges = create_gpu_readonly_storage(ranges_gpu.as_slice(), gpu);
    let sum_all_count = create_gpu_readonly_storage(&sum_all_count_host, gpu);

    let draw_list = DeviceDrawList {
      id_pool: self.internal.pool.pool_buffer_readonly(),
      dispatch_info: MultiRangeDispatchInfo {
        sub_list_ranges,
        sum_all_count,
        host_capacity_ranges: capacity_ranges,
        sum_all_count_host,
      },
    };

    SceneModelRenderBatch::Device(Some(DeviceSceneModelDrawList {
      draw_list,
      impl_select_ids,
    }))
  }
}

/// Combined spawn-stage result: base pool update + pre-built sort sparse writes.
pub struct OccStyleOrderControlSceneBatchUpdates {
  pub pool_update: PoolAllocationUpdate,
  /// Pre-built sparse writes for sort reordering, already with pool offsets applied.
  pub sort_sparse_writes: SparseBufferWritesSource,
}

impl OccStyleOrderControlSceneBatchExtractor {
  pub fn prepare_updates(
    &mut self,
    query: impl DualQueryLike<Key = RawEntityHandle, Value = (OccSceneModelGroupKey, RawEntityHandle)>,
    priority_changes: impl Query<Key = RawEntityHandle, Value = ValueChange<u32>>,
    allocator: &Arc<RwLock<GrowableRangeAllocator<u64>>>,
  ) -> OccStyleOrderControlSceneBatchUpdates {
    let (view, delta) = query.view_delta();
    let (base_update, mut changed_keys) = self.internal.prepare_updates(delta, allocator);

    for (sm_id, _) in priority_changes.iter_key_value() {
      // here we can skip the check that the sm's (key, scene) change's previous value's list update.
      // because it's already been included in changed_keys.
      if let Some((key, scene)) = view.access(&sm_id) {
        changed_keys.insert((key, scene));
      } else {
        // this is possible because we impl visible filtering
      }
    }

    let priority_view = read_global_db_component::<SceneModelOccStylePriority>();
    let mut sort_sparse_writes = SparseBufferWritesSource::default();

    let alloc = allocator.read();

    for (scene_id, group_hash) in &base_update.groups_with_updates {
      if let Some(scene_groups) = self.internal.contents.get_mut(scene_id) {
        for buffer in scene_groups.values_mut() {
          if buffer.group_key_hash == *group_hash {
            if let Some(sort_writes) = sort_by_priority(buffer, &priority_view) {
              let offset = alloc.get_region(*group_hash).unwrap().1;
              for (pos, val) in &sort_writes {
                sort_sparse_writes.collect_write(bytes_of(val), (offset + pos) as u64 * 4);
              }
            }
            break;
          }
        }
      }
    }

    OccStyleOrderControlSceneBatchUpdates {
      pool_update: base_update.pool_update,
      sort_sparse_writes,
    }
  }

  pub fn do_updates(
    &mut self,
    updates: &OccStyleOrderControlSceneBatchUpdates,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    self
      .internal
      .pool
      .apply_pool_update(&updates.pool_update, gpu, encoder);

    updates
      .sort_sparse_writes
      .write_abstract(gpu, encoder, self.internal.pool.pool_buffer());
  }
}

// todo-optimize: take most used priority into group key to avoid sort large list every time
pub fn sort_by_priority(
  buffer: &mut PersistSceneModelListBuffer,
  read_view: &ComponentReadView<SceneModelOccStylePriority>,
) -> Option<Vec<(u32, u32)>> {
  let host_before = buffer.host.clone();
  buffer.host.sort_by_cached_key(|handle| {
    unsafe { read_view.get_by_untyped_handle(*handle) }
      .copied()
      .unwrap_or(0)
  });
  let mut writes = Vec::new();
  for (i, (new, old)) in buffer.host.iter().zip(host_before.iter()).enumerate() {
    if new.index() != old.index() {
      writes.push((i as u32, new.alloc_index()));
    }
  }
  (!writes.is_empty()).then_some(writes)
}
