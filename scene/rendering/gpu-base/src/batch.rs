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

#[derive(Clone)]
pub struct HostModelLookUp {
  pub v: RevRefOfForeignKey<SceneModelBelongsToScene>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  pub scene_id: EntityHandle<SceneEntity>,
}

impl HostRenderBatch for HostModelLookUp {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    let iter = self.v.access_multi_value_dyn(&self.scene_id).filter(|sm| {
      let node = self.sm_ref_node.get(*sm).unwrap();
      self.node_net_visible.access(&node).unwrap_or(false)
    });
    Box::new(iter)
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

pub trait AbstractCullerProvider: ShaderHashProvider + DynClone {
  fn create_invocation(&self) -> Box<dyn AbstractCullerInvocation>;
  fn bind(&self);

  fn not(&self) -> Box<dyn AbstractCullerProvider> {
    todo!()
  }

  fn chain(&self, other: Box<dyn AbstractCullerProvider>) -> Box<dyn AbstractCullerProvider> {
    todo!()
  }
}
dyn_clone::clone_trait_object!(AbstractCullerProvider);

pub trait AbstractCullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool>;
}

impl DeviceSceneModelRenderBatch {
  pub fn with_override_culler(mut self, v: Box<dyn AbstractCullerProvider>) -> Self {
    self.stash_culler = Some(v);
    self
  }

  // todo, implement culling
  pub fn flush_culler(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Vec<DeviceSceneModelRenderSubBatch> {
    self.sub_batches.clone()
  }
}

// #[derive(Clone)]
// struct Filter {
//   last_frame: StorageBufferDataView<[Bool]>,
//   input: Box<dyn DeviceParallelComputeIO<u32>>,
// }

// impl DeviceParallelCompute<Node<u32>> for Filter {
//   fn execute_and_expose(
//     &self,
//     cx: &mut DeviceParallelComputeCtx,
//   ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
//     todo!()
//   }

//   fn result_size(&self) -> u32 {
//     todo!()
//   }
// }

// impl DeviceParallelComputeIO<u32> for Filter {}

// let sub_batches = batch
//   .sub_batches
//   .iter()
//   .map(|sub_batch| {
//     let scene_models = Box::new(Filter {
//       last_frame: last_frame.clone(),
//       input: sub_batch.scene_models.clone(),
//     });

//     DeviceSceneModelRenderSubBatch {
//       scene_models,
//       impl_select_id: sub_batch.impl_select_id,
//     }
//   })
//   .collect();

// DeviceSceneModelRenderBatch { sub_batches }

#[derive(Clone)]
pub struct NoopCuller;

impl ShaderHashProvider for NoopCuller {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for NoopCuller {
  fn create_invocation(&self) -> Box<dyn AbstractCullerInvocation> {
    todo!()
  }
  fn bind(&self) {}
}
