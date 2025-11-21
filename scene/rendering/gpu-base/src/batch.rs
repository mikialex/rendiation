use crate::*;

/// a logical batch of scene models
///
/// the models are reorderable currently, but may be configurable in future
#[derive(Clone)]
pub enum SceneModelRenderBatch {
  Device(DeviceSceneModelRenderBatch),
  Host(Box<dyn HostRenderBatch>),
}

pub trait HostRenderBatch: DynClone {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_>;
  fn materialize(&self) -> Vec<EntityHandle<SceneModelEntity>> {
    self.iter_scene_models().collect()
  }
}
#[derive(Clone)]
pub struct IteratorAsHostRenderBatch<T>(pub T);
impl<T> HostRenderBatch for IteratorAsHostRenderBatch<T>
where
  T: IntoIterator<Item = EntityHandle<SceneModelEntity>> + Clone,
{
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.0.clone().into_iter())
  }
}

impl HostRenderBatch for Arc<Vec<EntityHandle<SceneModelEntity>>> {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.iter().copied())
  }
}

dyn_clone::clone_trait_object!(HostRenderBatch);

// todo, we should make it incremental
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

#[derive(Clone)]
pub struct DeviceSceneModelRenderBatch {
  /// each sub batch could be and would be drawn by a multi-indirect-draw.
  pub sub_batches: Vec<DeviceSceneModelRenderSubBatch>,
  /// the culler for this batch, before the batch content be consumed/used, the culler
  /// must be consider by [`DeviceSceneModelRenderBatch::flush_culler`]
  ///
  /// The reason we have to keep the culler here because the culler logic is subject to compose
  /// with other cullers for example: [AbstractCullerProviderExt]. It's only possible if the culler
  /// is stored separately here.
  pub stash_culler: Option<Box<dyn AbstractCullerProvider>>,
}

impl DeviceSceneModelRenderBatch {
  pub fn empty() -> Self {
    Self {
      sub_batches: vec![],
      stash_culler: None,
    }
  }
}

/// todo, using this to improve the dispatch call count.
// #[derive(Clone)]
// pub struct DeviceSceneModelRenderBatchCombined {
//   pub scene_ids: StorageBufferDataView<[u32]>,
//   pub sub_batch_ranges: StorageBufferDataView<[Vec2<u32>]>,
// }

#[derive(Clone)]
pub struct DeviceSceneModelRenderSubBatch {
  pub scene_models: Box<dyn ComputeComponentIO<u32>>,
  /// this id is only used for implementation selecting. this may be not included in scene model.
  pub impl_select_id: EntityHandle<SceneModelEntity>,
  pub group_key: u64,
}

impl SceneModelRenderBatch {
  pub fn get_device_batch(&self) -> Option<DeviceSceneModelRenderBatch> {
    match self {
      SceneModelRenderBatch::Device(v) => Some(v.clone()),
      SceneModelRenderBatch::Host(_) => None,
    }
  }

  pub fn get_host_batch(&self) -> Option<Box<dyn HostRenderBatch>> {
    match self {
      SceneModelRenderBatch::Host(v) => Some(v.clone()),
      SceneModelRenderBatch::Device(_) => None,
    }
  }
}

impl DeviceSceneModelRenderBatch {
  pub fn set_override_culler(&mut self, v: impl AbstractCullerProvider + 'static) -> &mut Self {
    self.stash_culler = Some(Box::new(v));
    self
  }

  pub fn with_override_culler(mut self, v: impl AbstractCullerProvider + 'static) -> Self {
    self.stash_culler = Some(Box::new(v));
    self
  }

  pub fn flush_culler(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    require_materialize: bool,
  ) -> Vec<DeviceSceneModelRenderSubBatch> {
    if let Some(culler) = &self.stash_culler {
      cx.scope(|cx| {
        self
          .sub_batches
          .iter()
          .map(|sub_batch| {
            let mask = SceneModelCullingComponent {
              culler: culler.clone(),
              input: sub_batch.scene_models.clone(),
            };

            let scene_models = cx.keyed_scope(&sub_batch.group_key, |cx| {
              if require_materialize {
                let scene_models = sub_batch
                  .scene_models
                  .clone()
                  .stream_compaction(mask, cx)
                  .materialize_storage_buffer(cx);
                Box::new(scene_models) as Box<dyn ComputeComponentIO<u32>>
              } else {
                Box::new(sub_batch.scene_models.clone().stream_compaction(mask, cx))
              }
            });

            DeviceSceneModelRenderSubBatch {
              scene_models,
              impl_select_id: sub_batch.impl_select_id,
              group_key: sub_batch.group_key,
            }
          })
          .collect()
      })
    } else {
      self.sub_batches.clone()
    }
  }

  pub fn flush_culler_into_new(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    require_materialize: bool,
  ) -> Self {
    Self {
      sub_batches: self.flush_culler(cx, require_materialize),
      stash_culler: None,
    }
  }
}
