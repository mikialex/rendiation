use std::sync::Arc;

use bytemuck::bytes_of;
use database::*;
use fast_hash_collection::FastHashMap;
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
    .dual_query_intersect(layer) // interne impl filter out invisible, so here we use intersect
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
  let (cx, extractor) =
    cx.use_plain_state_default_cloned::<Arc<RwLock<OccStyleOrderControlSceneBatchExtractor>>>();

  cx.if_inspect(|inspector| {
    let bytes = extractor.read().internal.memory_usage();
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

    Some(Box::new(extractor.make_read_holder()))
  } else {
    None
  }
}

#[derive(Default)]
pub struct OccStyleOrderControlSceneBatchExtractor {
  internal: IncrementalDeviceSceneBatchExtractor<OccSceneModelGroupKey>,
}

impl SceneBatchBasicExtractAbility for OccStyleOrderControlSceneBatchExtractor {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    _renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    let contents = self.internal.contents.get(&scene.into_raw());
    if contents.is_none() {
      return SceneModelRenderBatch::Device(DeviceSceneModelRenderBatch::empty());
    }

    let contents = contents.unwrap();

    let mut sub_batches_with_key: Vec<_> =
      if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
        contents
          .iter()
          .filter(|(k, _)| k.internal.require_alpha_blend() == alpha_blend)
          .filter_map(|(k, v)| Some((v.create_batch()?, k.layer)))
          .collect()
      } else {
        contents
          .iter()
          .filter_map(|(k, v)| Some((v.create_batch()?, k.layer)))
          .collect()
      };

    sub_batches_with_key.sort_by_key(|v| v.1 as u32);

    let sub_batches = sub_batches_with_key.into_iter().map(|v| v.0).collect();

    let batches = DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    };
    SceneModelRenderBatch::Device(batches)
  }
}

impl OccStyleOrderControlSceneBatchExtractor {
  pub fn prepare_updates(
    &mut self,
    delta: impl Query<
      Key = RawEntityHandle,
      Value = ValueChange<(OccSceneModelGroupKey, RawEntityHandle)>,
    >,
  ) -> OccStyleOrderControlSceneBatchUpdates {
    let (updates, changed_keys) = self.internal.prepare_updates(delta);
    let mut sort_updates = FastHashMap::default();
    let priority_view = read_global_db_component::<SceneModelOccStylePriority>();
    for (key, scene) in &changed_keys {
      let list = self.internal.get_or_create(scene, key);
      if let Some(sort_write) = sort_by_priority(list, &priority_view) {
        sort_updates.insert((*scene, key.clone()), sort_write);
      }
    }
    OccStyleOrderControlSceneBatchUpdates {
      pre_updates: updates,
      sort_updates,
    }
  }

  pub fn do_updates(
    &mut self,
    updates: &OccStyleOrderControlSceneBatchUpdates,
    alloc: &dyn AbstractStorageAllocator,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    self
      .internal
      .do_updates(&updates.pre_updates, alloc, gpu, encoder);

    for ((scene_id, key), updates) in &updates.sort_updates {
      let list = self.internal.contents.get_mut(scene_id).unwrap();
      let list = list.get_mut(key).unwrap();

      let buffer = list.buffer.as_ref().unwrap();
      updates.write_abstract(gpu, encoder, &buffer.buffer);
    }
  }
}

pub struct OccStyleOrderControlSceneBatchUpdates {
  pre_updates: FastHashMap<(RawEntityHandle, OccSceneModelGroupKey), SparseBufferWritesSource>,
  sort_updates: FastHashMap<(RawEntityHandle, OccSceneModelGroupKey), SparseBufferWritesSource>,
}

// todo-optimize: we should take most used priority into group key to avoid sort large list every time
pub fn sort_by_priority(
  buffer: &mut PersistSceneModelListBuffer,
  read_view: &ComponentReadView<SceneModelOccStylePriority>,
) -> Option<SparseBufferWritesSource> {
  let host_before = buffer.host.clone();
  buffer.host.sort_by_cached_key(|handle| {
    unsafe { read_view.get_by_untyped_handle(*handle) }
      .copied()
      .unwrap_or(0);
  });
  let mut write_source = SparseBufferWritesSource::default();
  for (i, (new, old)) in buffer.host.iter().zip(host_before.iter()).enumerate() {
    if new.index() != old.index() {
      write_source.collect_write(
        bytes_of(&new.index()),
        (i * std::mem::size_of::<u32>()) as u64,
      );
    }
  }

  write_source.is_empty().then_some(write_source)
}
