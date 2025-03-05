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

impl HostRenderBatch for Vec<EntityHandle<SceneModelEntity>> {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>>> {
    Box::new(self.clone().into_iter())
  }
}

dyn_clone::clone_trait_object!(HostRenderBatch);

// todo, we should make it incremental
#[derive(Clone)]
pub struct HostModelLookUp {
  pub v: RevRefOfForeignKey<SceneModelBelongsToScene>,
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
          .unwrap_or(false)
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
  pub stash_culler: Option<Box<dyn AbstractCullerProvider>>,
}

#[derive(Clone)]
pub struct DeviceSceneModelRenderSubBatch {
  pub scene_models: Box<dyn DeviceParallelComputeIO<u32>>,
  /// this id is only used for implementation selecting. this may be not included in scene model.
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
    // todo use indirect grouper for safeness
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
            sub_batches: vec![DeviceSceneModelRenderSubBatch {
              impl_select_id: v.iter_scene_models().next().unwrap(),
              scene_models: Box::new(storage),
            }],
            stash_culler: None,
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

impl DeviceSceneModelRenderBatch {
  pub fn with_override_culler(mut self, v: Box<dyn AbstractCullerProvider>) -> Self {
    self.stash_culler = Some(v);
    self
  }

  pub fn flush_culler(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Vec<DeviceSceneModelRenderSubBatch> {
    if let Some(culler) = &self.stash_culler {
      self
        .sub_batches
        .iter()
        .map(|sub_batch| {
          let mask = SceneModelCulling {
            culler: culler.clone(),
            input: sub_batch.scene_models.clone(),
          };

          DeviceSceneModelRenderSubBatch {
            scene_models: Box::new(sub_batch.scene_models.clone().stream_compaction(mask)),
            impl_select_id: sub_batch.impl_select_id,
          }
        })
        .collect()
    } else {
      self.sub_batches.clone()
    }
  }

  pub fn flush_culler_into_new(&self, cx: &mut DeviceParallelComputeCtx) -> Self {
    Self {
      sub_batches: self.flush_culler(cx),
      stash_culler: None,
    }
  }
}
