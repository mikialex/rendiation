use crate::*;

pub trait IndirectNodeRenderImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneNodeEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<SceneNodeEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;
}

#[derive(Default)]
pub struct DefaultIndirectNodeRenderImplProvider {
  storage: QueryToken,
}
pub struct DefaultIndirectNodeRenderImpl {
  node_gpu: LockReadGuardHolder<MultiUpdateContainer<CommonStorageBufferImpl<NodeStorage>>>,
}

impl QueryBasedFeature<Box<dyn IndirectNodeRenderImpl>> for DefaultIndirectNodeRenderImplProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let storage = node_storages(cx);
    self.storage = qcx.register_multi_updater(storage);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.storage);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectNodeRenderImpl> {
    Box::new(DefaultIndirectNodeRenderImpl {
      node_gpu: cx.take_multi_updater_updated(self.storage).unwrap(),
    })
  }
}

impl IndirectNodeRenderImpl for DefaultIndirectNodeRenderImpl {
  fn make_component_indirect(
    &self,
    _any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUStorage {
      buffer: &self.node_gpu,
    };
    Some(Box::new(node))
  }
  fn hash_shader_group_key(
    &self,
    _: EntityHandle<SceneNodeEntity>,
    _: &mut PipelineHasher,
  ) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
