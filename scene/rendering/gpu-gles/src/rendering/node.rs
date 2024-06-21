use crate::*;

pub trait GLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;
}

#[derive(Default)]
pub struct DefaultGLESNodeRenderImplProvider {
  uniforms: UpdateResultToken,
}
pub struct DefaultGLESNodeRenderImpl {
  node_gpu: LockReadGuardHolder<SceneNodeUniforms>,
}

impl RenderImplProvider<Box<dyn GLESNodeRenderImpl>> for DefaultGLESNodeRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPUResourceCtx) {
    let uniforms = node_gpus(cx);
    self.uniforms = source.register_multi_updater(uniforms);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn GLESNodeRenderImpl> {
    Box::new(DefaultGLESNodeRenderImpl {
      node_gpu: res.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

impl GLESNodeRenderImpl for DefaultGLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPU {
      ubo: self.node_gpu.get(&idx)?,
    };
    Some(Box::new(node))
  }
}
