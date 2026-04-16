use crate::*;

pub fn use_occ_host_scene_batch_extractor(
  cx: &mut QueryGPUHookCx,
) -> Option<Box<dyn SceneBatchBasicExtractAbility>> {
  use_default_scene_batch_extractor(cx).map(|internal| {
    let priority = read_global_db_component::<SceneModelOccStylePriority>();
    let layer = read_global_db_component::<SceneModelOccStyleLayer>();

    Box::new(Impl {
      internal,
      layer,
      priority,
    }) as Box<dyn SceneBatchBasicExtractAbility>
  })
}

struct Impl {
  internal: DefaultSceneBatchExtractor,
  layer: ComponentReadView<SceneModelOccStyleLayer>,
  priority: ComponentReadView<SceneModelOccStylePriority>,
}

impl SceneBatchBasicExtractAbility for Impl {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    let mut sm: Vec<_> = self
      .internal
      .extract(scene, semantic)
      .iter_scene_models()
      .collect();

    sm.sort_by_cached_key(|&sm| {
      let layer = self.layer.get(sm).copied().unwrap_or_default() as u32;
      let priority = self.priority.get(sm).copied().unwrap_or_default();
      let layer = (layer as u64) << 32;
      let priority = priority as u64;
      layer & priority
    });

    let batch = IteratorAsHostRenderBatch(sm.into_iter());

    if let Some(creator) = renderer.indirect_batch_direct_creator() {
      SceneModelRenderBatch::Device(creator.create_batch_from_iter(&mut batch.iter_scene_models()))
    } else {
      SceneModelRenderBatch::Host(Box::new(batch))
    }
  }
}
