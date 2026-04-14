use crate::*;

pub type IncrementalDeviceSceneBatchExtractorShared<K> =
  Arc<RwLock<IncrementalDeviceSceneBatchExtractor<K>>>;

type GroupKeyWithSceneHandle<K> = (K, RawEntityHandle);

pub struct IncrementalDeviceSceneBatchExtractor<K> {
  contents: FastHashMap<RawEntityHandle, FastHashMap<K, PersistSceneModelListBuffer>>,
}

impl<K> Default for IncrementalDeviceSceneBatchExtractor<K> {
  fn default() -> Self {
    Self {
      contents: Default::default(),
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
        let mut key_hasher = FastHasher::default();
        key.hash(&mut key_hasher);
        let hash = key_hasher.finish();

        (
          key.clone(),
          PersistSceneModelListBuffer::with_capacity(1024, hash),
        )
      })
      .1
  }
}

pub struct IncrementalDeviceSceneBatchExtractorUpdates<K> {
  updates: FastHashMap<RawEntityHandle, FastHashMap<K, PersistSceneModelListBufferMutation>>,
}

impl<K> Default for IncrementalDeviceSceneBatchExtractorUpdates<K> {
  fn default() -> Self {
    Self {
      updates: Default::default(),
    }
  }
}

impl<K: Eq + Hash + Clone> IncrementalDeviceSceneBatchExtractorUpdates<K> {
  fn get_or_create_source(
    &mut self,
    scene: &RawEntityHandle,
    key: &K,
    list: &PersistSceneModelListBuffer,
  ) -> &mut PersistSceneModelListBufferMutation {
    self
      .updates
      .raw_entry_mut()
      .from_key(scene)
      .or_insert_with(|| (*scene, Default::default()))
      .1
      .raw_entry_mut()
      .from_key(key)
      .or_insert_with(|| (key.clone(), list.create_mutation()))
      .1
  }
}

pub struct IncrementalDeviceSceneBatchExtractorGPUUpdates<K> {
  updates: FastHashMap<RawEntityHandle, FastHashMap<K, SparseBufferWritesSource>>,
}

impl<K: Eq + Hash + Clone> IncrementalDeviceSceneBatchExtractor<K> {
  pub fn prepare_updates(
    &mut self,
    delta: impl Query<Key = RawEntityHandle, Value = ValueChange<GroupKeyWithSceneHandle<K>>>,
  ) -> IncrementalDeviceSceneBatchExtractorGPUUpdates<K> {
    let mut updates = IncrementalDeviceSceneBatchExtractorUpdates::<K>::default();
    for (sm, key_change) in delta.iter_key_value() {
      if let Some((key, scene_id)) = key_change.old_value() {
        let list = self.get_or_create(scene_id, key);
        let updates = updates.get_or_create_source(scene_id, key, list);
        list.remove(sm, updates);
      }

      if let Some((key, scene_id)) = key_change.new_value() {
        let list = self.get_or_create(scene_id, key);
        let updates = updates.get_or_create_source(scene_id, key, list);
        list.insert(sm, updates);
      }
    }

    let updates = updates
      .updates
      .into_iter()
      .map(|(k, v)| {
        let v = v
          .into_iter()
          .filter_map(|(k, v)| (k, v.into_sparse_update()?).into())
          .collect();
        (k, v)
      })
      .collect();

    IncrementalDeviceSceneBatchExtractorGPUUpdates { updates }
  }

  pub fn do_updates(
    &mut self,
    updates: &IncrementalDeviceSceneBatchExtractorGPUUpdates<K>,
    alloc: &dyn AbstractStorageAllocator,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    for (scene_id, updates) in &updates.updates {
      let list = self.contents.get_mut(scene_id).unwrap();
      for (key, updates) in updates {
        let list = list.get_mut(key).unwrap();
        list.update_gpu(alloc, gpu, encoder, updates);
      }
    }
  }
}

impl IncrementalDeviceSceneBatchExtractor<SceneModelGroupKey> {
  pub fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
  ) -> Option<SceneModelRenderBatch> {
    let contents = self.contents.get(&scene.into_raw())?;
    let sub_batches = if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
      contents
        .iter()
        .filter(|(k, _)| k.require_alpha_blend() == alpha_blend)
        .filter_map(|(_, v)| v.create_batch())
        .collect()
    } else {
      contents.values().filter_map(|v| v.create_batch()).collect()
    };
    let batches = DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    };
    SceneModelRenderBatch::Device(batches).into()
  }
}
