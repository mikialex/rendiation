use crate::*;

pub fn use_default_scene_batch_extractor(
  cx: &mut impl QueryGPUHookCx,
) -> Option<DefaultSceneBatchExtractor> {
  let model_lookup = cx.use_global_multi_reactive_query::<SceneModelBelongsToScene>();

  let node_net_visible = cx.use_reactive_query(scene_node_derive_visible);
  let model_alpha_blend = cx.use_reactive_query(all_kinds_of_materials_enabled_alpha_blending);

  cx.when_render(|| DefaultSceneBatchExtractor {
    model_lookup: model_lookup.unwrap(),
    node_net_visible: node_net_visible.unwrap(),
    alpha_blend: model_alpha_blend.unwrap(),
    sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
  })
}

pub struct DefaultSceneBatchExtractor {
  node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  alpha_blend: BoxedDynQuery<EntityHandle<SceneModelEntity>, bool>,
  model_lookup: RevRefOfForeignKey<SceneModelBelongsToScene>,
}

impl DefaultSceneBatchExtractor {
  pub fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
    _ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch {
    SceneModelRenderBatch::Host(Box::new(HostModelLookUp {
      v: self.model_lookup.clone(),
      node_net_visible: self.node_net_visible.clone(),
      sm_ref_node: self.sm_ref_node.clone(),
      scene_id: scene,
      scene_model_use_alpha_blending: self.alpha_blend.clone(),
      enable_alpha_blending: semantic.only_alpha_blend_objects,
    }))
  }
}
