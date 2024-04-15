use crate::*;

pub trait GLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>>;
}

pub struct DefaultGLESNodeRenderImplProvider;
pub struct DefaultGLESNodeRenderImpl {
  node_gpu: LockReadGuardHolder<SceneNodeUniforms>,
}

impl RenderImplProvider<Box<dyn GLESNodeRenderImpl>> for DefaultGLESNodeRenderImplProvider {
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    let uniforms = node_gpus(res.cx());
    res.register_multi_updater(uniforms);
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESNodeRenderImpl> {
    Box::new(DefaultGLESNodeRenderImpl {
      node_gpu: res.get_multi_updater().unwrap(),
    })
  }
}

impl GLESNodeRenderImpl for DefaultGLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    let node = NodeGPU {
      ubo: self.node_gpu.get(&idx)?,
    };
    Some(Box::new(node))
  }
}
