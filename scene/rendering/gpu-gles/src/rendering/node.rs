use crate::*;

pub trait GLESNodeRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;
}

pub fn use_node_uniforms(cx: &mut QueryGPUHookCx) -> Option<DefaultGLESNodeRenderImpl> {
  cx.use_uniform_buffers(|source, cx| {
    source.with_source(
      scene_node_derive_world_mat()
        .collective_map(|mat| NodeUniform::from_world_mat(mat))
        .into_query_update_uniform(0, cx),
    )
  })
  .map(|node_gpu| DefaultGLESNodeRenderImpl { node_gpu })
}

pub struct DefaultGLESNodeRenderImpl {
  node_gpu: LockReadGuardHolder<SceneNodeUniforms>,
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
