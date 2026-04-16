use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SceneContentKey {
  pub only_alpha_blend_objects: Option<bool>,
}

impl SceneContentKey {
  pub fn only_opaque_objects() -> Self {
    Self {
      only_alpha_blend_objects: Some(false),
    }
  }
  pub fn only_alpha_blend_objects() -> Self {
    Self {
      only_alpha_blend_objects: Some(true),
    }
  }
}

pub trait SceneBatchBasicExtractAbility {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch;
}

impl<T: SceneBatchBasicExtractAbility> SceneBatchBasicExtractAbility for LockReadGuardHolder<T> {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    (**self).extract_scene_batch(scene, semantic, renderer)
  }
}

pub fn use_default_scene_batch_extractor(
  cx: &mut QueryGPUHookCx<'_>,
) -> Option<DefaultSceneBatchExtractor> {
  let model_lookup = cx.use_db_rev_ref_typed::<SceneModelBelongsToScene>();

  let node_net_visible = use_global_node_net_visible(cx).use_assure_result(cx);

  cx.when_render(|| DefaultSceneBatchExtractor {
    model_lookup: model_lookup.expect_resolve_stage(),
    node_net_visible: node_net_visible
      .expect_resolve_stage()
      .view()
      .mark_entity_type::<SceneNodeEntity>()
      .into_boxed(),
    alpha_blend: all_kinds_of_materials_enabled_alpha_blending().into_boxed(),
    sm_ref_node: read_global_db_foreign_key(),
  })
}

pub struct DefaultSceneBatchExtractor {
  node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  alpha_blend: BoxedDynQuery<EntityHandle<SceneModelEntity>, bool>,
  model_lookup: RevRefForeignKeyReadTyped<SceneModelBelongsToScene>,
}

impl DefaultSceneBatchExtractor {
  pub fn extract(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
  ) -> HostModelLookUp {
    HostModelLookUp {
      v: self.model_lookup.clone(),
      node_net_visible: self.node_net_visible.clone(),
      sm_ref_node: self.sm_ref_node.clone(),
      scene_id: scene,
      scene_model_use_alpha_blending: self.alpha_blend.clone(),
      enable_alpha_blending: semantic.only_alpha_blend_objects,
    }
  }
}

impl SceneBatchBasicExtractAbility for DefaultSceneBatchExtractor {
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    renderer: &dyn SceneRenderer,
  ) -> SceneModelRenderBatch {
    let batch = self.extract(scene, semantic);

    if let Some(creator) = renderer.indirect_batch_direct_creator() {
      SceneModelRenderBatch::Device(creator.create_batch_from_iter(&mut batch.iter_scene_models()))
    } else {
      SceneModelRenderBatch::Host(Box::new(batch))
    }
  }
}

#[derive(Clone)]
pub struct HostModelLookUp {
  pub v: RevRefForeignKeyReadTyped<SceneModelBelongsToScene>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_use_alpha_blending: BoxedDynQuery<EntityHandle<SceneModelEntity>, bool>,
  pub sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  pub scene_id: EntityHandle<SceneEntity>,
  pub enable_alpha_blending: Option<bool>,
}

impl HostRenderBatch for HostModelLookUp {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    let iter = self.v.access_multi_value_dyn(&self.scene_id).filter(|sm| {
      let node = self.sm_ref_node.get(*sm).unwrap();
      self.node_net_visible.access(&node).unwrap_or(false)
    });

    if let Some(enable_alpha_blending) = self.enable_alpha_blending {
      let iter = iter.filter(move |sm| {
        self
          .scene_model_use_alpha_blending
          .access(sm)
          .unwrap_or(false) // todo, is this right?
          == enable_alpha_blending
      });
      Box::new(iter)
    } else {
      Box::new(iter)
    }
  }
}
