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
dyn_clone::clone_trait_object!(HostRenderBatch);

impl HostRenderBatch for Vec<EntityHandle<SceneModelEntity>> {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    Box::new(self.iter().copied())
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

#[derive(Clone)]
pub struct DeviceSceneModelRenderBatch {
  /// each sub batch could be and would be drawn by a multi-indirect-draw.
  pub sub_batches: Vec<DeviceSceneModelRenderSubBatch>,
}

impl DeviceSceneModelRenderBatch {
  pub fn empty() -> Self {
    Self {
      sub_batches: vec![],
    }
  }
}

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
  /// require_fully_materialize is to ensure the result list has no reference relation to the self.
  #[track_caller]
  pub fn execute_culling(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    culler: Box<dyn AbstractCullerProvider>,
    require_fully_materialize: bool,
  ) -> Self {
    let sub_batches = self
      .sub_batches
      .iter()
      .map(|sub_batch| {
        let mask = SceneModelCullingComponent {
          culler: culler.clone(),
          input: sub_batch.scene_models.clone(),
        };

        cx.next_key_scope_root();
        let scene_models = cx.keyed_scope(&sub_batch.group_key, |cx| {
          if require_fully_materialize {
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
      .collect();

    Self { sub_batches }
  }
}
