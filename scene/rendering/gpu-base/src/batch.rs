use crate::*;

#[derive(Clone)]
pub enum SceneModelRenderBatch {
  Device(DeviceSceneModelRenderBatch),
  Host(Box<dyn HostRenderBatch>),
}

pub trait HostRenderBatch: DynClone {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_>;
}

impl HostRenderBatch for Vec<EntityHandle<SceneModelEntity>> {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>>> {
    Box::new(self.clone().into_iter())
  }
}

dyn_clone::clone_trait_object!(HostRenderBatch);

#[derive(Clone)]
pub struct DeviceSceneModelRenderBatch {
  pub scene_models: Box<dyn DeviceParallelComputeIO<u32>>,
  pub impl_select_id: EntityHandle<SceneModelEntity>,
}

impl SceneModelRenderBatch {
  /// user must assure the given host batch could be converted to device batch logically correct.
  /// (could be rendered indirectly, and at least has one scene model)
  ///
  /// **warning**, convert device to host may affect performance if scene model list is large
  pub fn get_device_batch(
    &self,
    force_convert: Option<&GPU>,
  ) -> Option<DeviceSceneModelRenderBatch> {
    match self {
      SceneModelRenderBatch::Device(v) => Some(v.clone()),
      SceneModelRenderBatch::Host(v) => {
        if let Some(gpu) = force_convert {
          let data = v
            .iter_scene_models()
            .map(|v| v.alloc_index())
            .collect::<Vec<_>>();
          let storage = create_gpu_readonly_storage(data.as_slice(), &gpu.device);
          Some(DeviceSceneModelRenderBatch {
            scene_models: Box::new(storage),
            impl_select_id: v.iter_scene_models().next().unwrap(),
          })
        } else {
          None
        }
      }
    }
  }

  pub fn get_host_batch(&self) -> Option<Box<dyn HostRenderBatch>> {
    match self {
      SceneModelRenderBatch::Host(v) => Some(v.clone()),
      SceneModelRenderBatch::Device(_) => None,
    }
  }
}
