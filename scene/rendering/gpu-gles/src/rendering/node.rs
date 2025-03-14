use crate::*;

pub trait GLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;
}

#[derive(Default)]
pub struct DefaultGLESNodeRenderImplProvider {
  uniforms: QueryToken,
}
pub struct DefaultGLESNodeRenderImpl {
  node_gpu: LockReadGuardHolder<SceneNodeUniforms>,
}

impl QueryBasedFeature<Box<dyn GLESNodeRenderImpl>> for DefaultGLESNodeRenderImplProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let uniforms = node_uniforms(cx);
    self.uniforms = qcx.register_multi_updater(uniforms);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.uniforms);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn GLESNodeRenderImpl> {
    Box::new(DefaultGLESNodeRenderImpl {
      node_gpu: cx.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

impl GLESNodeRenderImpl for DefaultGLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = NodeGPUUniform {
      ubo: self.node_gpu.get(&idx)?,
    };
    Some(Box::new(node))
  }
}
