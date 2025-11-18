use crate::*;

pub type IncrementalDeviceSceneBatchExtractorShared =
  Arc<RwLock<IncrementalDeviceSceneBatchExtractor>>;

type GroupKeyWithSceneHandle = (SceneModelGroupKey, RawEntityHandle);

#[derive(Default)]
pub struct IncrementalDeviceSceneBatchExtractor {
  contents: FastHashMap<
    EntityHandle<SceneEntity>,
    FastHashMap<SceneModelGroupKey, PersistSceneModelListBuffer>,
  >,
}

pub struct IncrementalDeviceSceneBatchExtractorUpdates {
  updates: FastHashMap<
    EntityHandle<SceneEntity>,
    FastHashMap<SceneModelGroupKey, SparseBufferWritesSource>,
  >,
}

impl IncrementalDeviceSceneBatchExtractor {
  pub fn prepare_updates(
    &mut self,
    delta: impl Query<Key = RawEntityHandle, Value = ValueChange<GroupKeyWithSceneHandle>>,
  ) -> IncrementalDeviceSceneBatchExtractorUpdates {
    for (sm, key_change) in delta.iter_key_value() {
      match key_change {
        ValueChange::Delta(_, _) => todo!(),
        ValueChange::Remove(_) => todo!(),
      }
    }
    //

    todo!()
  }

  pub fn do_updates(
    &mut self,
    updates: &IncrementalDeviceSceneBatchExtractorUpdates,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
  ) {
    todo!()
  }

  pub fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
  ) -> SceneModelRenderBatch {
    let contents = self.contents.get(&scene).unwrap();
    let sub_batches = if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
      contents
        .iter()
        .filter(|(k, _)| k.require_alpha_blend() == alpha_blend)
        .map(|(_, v)| v.create_batch())
        .collect()
    } else {
      contents.values().map(|v| v.create_batch()).collect()
    };
    let batches = DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    };
    SceneModelRenderBatch::Device(batches)
  }
}
