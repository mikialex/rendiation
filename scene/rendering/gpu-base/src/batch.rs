use crate::*;

/// a list of scene models
///
/// the models are reorderable currently, but may be configurable in future
#[derive(Clone)]
pub enum SceneModelRenderBatch {
  /// the none case means empty device list, as gpu layer not allow zero length buffer
  Device(Option<DeviceSceneModelDrawList>),
  Host(Box<dyn HostRenderBatch>),
}

impl SceneModelRenderBatch {
  pub fn get_device_batch(&self) -> Option<Option<DeviceSceneModelDrawList>> {
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

#[derive(Clone)]
pub struct DeviceSceneModelDrawList {
  pub draw_list: DeviceDrawList,
  /// this id is only used for implementation selecting. itself may be not included in list.
  pub impl_select_ids: Vec<EntityHandle<SceneModelEntity>>,
}

impl DeviceSceneModelDrawList {
  pub fn use_culled_list_and_do_culling(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    culler: Box<dyn AbstractCullerProvider>,
  ) -> Self {
    let draw_list_culled = self.draw_list.use_culled_list_and_do_culling(cx, culler);
    DeviceSceneModelDrawList {
      draw_list: draw_list_culled,
      impl_select_ids: self.impl_select_ids.clone(),
    }
  }
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
