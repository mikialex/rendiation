use std::hash::Hash;
use std::sync::Arc;

use crate::*;

pub type IncrementalDeviceSceneBatchExtractorShared<K> =
  Arc<RwLock<IncrementalDeviceSceneBatchExtractor<K>>>;

type GroupKeyWithSceneHandle<K> = (K, RawEntityHandle);

pub struct IncrementalDeviceSceneBatchExtractor<K> {
  pub contents: FastHashMap<RawEntityHandle, FastHashMap<K, PersistSceneModelListBuffer>>,
  pub pool: SceneModelListPool<(RawEntityHandle, K)>,
}

/// Snapshot after spawn-stage: entity changes + pool allocation update.
pub struct ExtractorUpdate<K> {
  pub groups_with_updates: Vec<(RawEntityHandle, K)>, // (scene_id, key)
  pub pool_update: PoolAllocationUpdate<(RawEntityHandle, K)>,
}

impl<K> IncrementalDeviceSceneBatchExtractor<K> {
  pub fn new(pool: SceneModelListPool<(RawEntityHandle, K)>) -> Self {
    Self {
      contents: FastHashMap::default(),
      pool,
    }
  }
}

impl<K: Eq + Hash + Clone> IncrementalDeviceSceneBatchExtractor<K> {
  pub fn memory_usage(&self) -> usize {
    self
      .contents
      .values()
      .map(|group| {
        group
          .values()
          .map(|buffer| buffer.memory_usage())
          .sum::<usize>()
      })
      .sum::<usize>()
      + self.contents.allocation_size()
  }

  fn remove_empty(&mut self, scene: &RawEntityHandle, key: &K) {
    let lists_of_scene = self.contents.get_mut(scene).unwrap();
    lists_of_scene.remove(key).unwrap();
    if lists_of_scene.is_empty() {
      self.contents.remove(scene).unwrap();
    }
  }

  fn get_or_create(
    &mut self,
    scene: &RawEntityHandle,
    key: &K,
  ) -> &mut PersistSceneModelListBuffer {
    self
      .contents
      .raw_entry_mut()
      .from_key(scene)
      .or_insert_with(|| (*scene, Default::default()))
      .1
      .raw_entry_mut()
      .from_key(key)
      .or_insert_with(|| {
        (
          key.clone(),
          PersistSceneModelListBuffer::with_capacity(1024),
        )
      })
      .1
  }
}

impl<K: Eq + Hash + Clone> IncrementalDeviceSceneBatchExtractor<K> {
  /// Spawn-stage: process entity insert/remove, prepare pool allocation update.
  pub fn prepare_updates(
    &mut self,
    delta: impl Query<Key = RawEntityHandle, Value = ValueChange<GroupKeyWithSceneHandle<K>>>,
  ) -> (ExtractorUpdate<K>, FastHashSet<(K, RawEntityHandle)>) {
    let mut changes_keys = FastHashSet::default();

    // Track old group capacities before changes
    let mut old_capacities = FastHashMap::default();

    for (sm, key_change) in delta.iter_key_value() {
      if let Some((key, scene_id)) = key_change.old_value() {
        changes_keys.insert((key.clone(), *scene_id));
        let list = self.get_or_create(scene_id, key);
        // Record old capacity before removal
        old_capacities
          .entry((*scene_id, key.clone()))
          .or_insert(list.host.len() as u32);
        list.remove(sm);
      }

      if let Some((key, scene_id)) = key_change.new_value() {
        changes_keys.insert((key.clone(), *scene_id));
        let list = self.get_or_create(scene_id, key);
        old_capacities
          .entry((*scene_id, key.clone()))
          .or_insert(list.host.len() as u32);
        list.insert(sm);
      }
    }

    // Build changed groups list for allocator update
    let mut groups_with_updates: Vec<(RawEntityHandle, K)> = Vec::new();
    let mut changed_groups: Vec<((RawEntityHandle, K), Vec<u8>, u32)> = Vec::new();
    let mut removed_groups: Vec<(RawEntityHandle, K)> = Vec::new();
    let mut entity_writes: Vec<((RawEntityHandle, K), u32, u32)> = Vec::new();

    let limits = &self.pool.gpu().info.supported_limits;
    let min_size_round_up = limits
      .min_storage_buffer_offset_alignment
      .max(limits.min_uniform_buffer_offset_alignment)
      / 4;

    for (key, s_id) in &changes_keys {
      let buffer = self.get_or_create(s_id, key);
      let new_size = buffer.host.len() as u32;

      let alloc_key = (*s_id, key.clone());

      if new_size == 0 {
        removed_groups.push(alloc_key);
        self.remove_empty(s_id, key);
        continue;
      }
      // the code here is cold path

      let new_size_rounded = new_size.next_power_of_two().max(min_size_round_up);

      // if capacity not changed, the data write is processed by sparse write
      let data: Vec<u8> = buffer
        .host
        .iter()
        .flat_map(|v| v.index().bytes().to_vec())
        .collect();

      let old_size = old_capacities.get(&alloc_key).copied().unwrap();
      // handle the edge case: new group has no old allocation to compare
      if old_size == 0 {
        changed_groups.push((alloc_key.clone(), data, new_size_rounded));
      } else {
        let old_size_rounded = old_size.next_power_of_two().max(min_size_round_up);

        if old_size_rounded != new_size_rounded {
          changed_groups.push((alloc_key.clone(), data, new_size_rounded));
        }
      }

      // Collect entity writes
      if let Some(updates) = buffer.updates.take() {
        for (pos, val) in updates.mapping_change {
          entity_writes.push((alloc_key.clone(), pos as u32, val));
        }
      }

      groups_with_updates.push(alloc_key);
    }

    let pool_update =
      self
        .pool
        .prepare_pool_update(&removed_groups, &changed_groups, entity_writes);

    if let Some(new_capacity) = pool_update.allocation_result.resize_to {
      self.pool.update_pool_size(new_capacity);
    }

    (
      ExtractorUpdate {
        groups_with_updates,
        pool_update,
      },
      changes_keys,
    )
  }

  /// Render-stage: apply pool allocation changes and write GPU data.
  pub fn do_updates(
    &mut self,
    update: &ExtractorUpdate<K>,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    self
      .pool
      .apply_pool_update(&update.pool_update, gpu, encoder);
  }
}

impl SceneBatchBasicExtractAbility for IncrementalDeviceSceneBatchExtractor<SceneModelGroupKey> {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    _renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    let contents = self.contents.get(&scene.into_raw());
    let Some(contents) = contents else {
      return SceneModelRenderBatch::Device(None);
    };

    let groups: Vec<_> = if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
      contents
        .iter()
        .filter(|(k, _)| k.require_alpha_blend() == alpha_blend)
        .collect()
    } else {
      contents.iter().collect()
    };

    if groups.is_empty() {
      return SceneModelRenderBatch::Device(None);
    }

    let mut impl_select_ids = Vec::with_capacity(groups.len());
    let mut host_capacity_ranges = Vec::with_capacity(groups.len());
    let mut real_lengths = Vec::with_capacity(groups.len());

    let alloc = self.pool.allocator.read();
    for (key, buffer) in &groups {
      impl_select_ids.push(buffer.representative().unwrap());
      real_lengths.push(buffer.host.len() as u32);
      let alloc_key = (scene.into_raw(), (*key).clone());
      let (capacity, offset) = alloc.get_region(&alloc_key).unwrap();
      host_capacity_ranges.push(CapacityRange { capacity, offset });
    }
    drop(alloc);

    let total_capacity: u32 = groups.iter().map(|(_, buf)| buf.host.len() as u32).sum();
    let gpu = self.pool.gpu();
    let ranges_gpu = prepare_gpu_sub_list_ranges(&host_capacity_ranges, &real_lengths);
    let device_ranges = DeviceMultiRangeDispatchInfo::new(gpu, ranges_gpu.as_slice());

    let draw_list = DeviceDrawList {
      id_pool: self.pool.pool_buffer_readonly(),
      dispatch_info: MultiRangeDispatchInfo {
        device_ranges,
        host_capacity_ranges,
        total_capacity,
      },
    };

    SceneModelRenderBatch::Device(Some(DeviceSceneModelDrawList {
      draw_list,
      impl_select_ids,
    }))
  }
}
