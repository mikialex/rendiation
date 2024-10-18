use crate::*;

pub trait IndirectNodeRenderImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;
}

#[derive(Default)]
pub struct DefaultIndirectNodeRenderImplProvider {
  storage: UpdateResultToken,
}
pub struct DefaultIndirectNodeRenderImpl {
  node_gpu: LockReadGuardHolder<MultiUpdateContainer<CommonStorageBufferImpl<NodeStorage>>>,
}

impl RenderImplProvider<Box<dyn IndirectNodeRenderImpl>> for DefaultIndirectNodeRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let storage = node_storages(cx);
    self.storage = source.register_multi_updater(storage.inner);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn IndirectNodeRenderImpl> {
    Box::new(DefaultIndirectNodeRenderImpl {
      node_gpu: res.take_multi_updater_updated(self.storage).unwrap(),
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
}