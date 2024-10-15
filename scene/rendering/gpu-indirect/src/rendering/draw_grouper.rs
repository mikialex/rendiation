use crate::*;

pub trait IndirectSceneDrawBatchGrouper {
  fn iter_grouped_scene_model(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn Iterator<Item = (Box<dyn IndirectBatchSource>, EntityHandle<SceneModelEntity>)>>;
}

// pub type PipelineVariantKey = u64;

// struct IndirectSceneDrawBatchGrouperImpl {
//   /// for performance reason(the scene model may exceed millions), the grouped dispatch source
//   /// buffer should be maintained incrementally
//   dispatch_scene_models_source: Vec<StorageBufferReadOnlyDataView<[u32]>>,

//   /// to maintained the dispatch groups, the dispatch group key multi-access-mapping scene-model-id is maintained
//   scene_models_dispatch_groups:
//     Box<dyn DynVirtualMultiCollection<PipelineVariantKey, EntityHandle<SceneModelEntity>>>,

//   scene_model_pipeline_key:
//     Box<dyn DynVirtualCollection<EntityHandle<SceneModelEntity>, PipelineVariantKey>>,

//   dispatch_group_renderer:
//     FastHashMap<PipelineVariantKey, Box<dyn IndirectBatchSceneModelRenderer>>,

//   model_lookup: RevRefOfForeignKey<SceneModelBelongsToScene>,
// }
