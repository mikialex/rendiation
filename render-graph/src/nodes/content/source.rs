use crate::{
  ContentProvider, NodeBuilder, RenderGraph, RenderGraphBackend, RenderGraphExecutor,
  RenderGraphNode, RenderGraphNodeHandle,
};

pub struct ContentSourceNodeData<T: RenderGraphBackend> {
  pub key: T::ContentSourceKey,
}

impl<T: RenderGraphBackend> ContentSourceNodeData<T> {
  pub fn execute(
    &self,
    self_handle: RenderGraphNodeHandle<T>,
    executor: &mut RenderGraphExecutor<T>,
    content_provider: &mut T::ContentProviderImpl,
  ) {
    content_provider.get_source(
      self.key,
      &executor.target_pool,
      executor.content_pool.active(self_handle),
    )
  }
}

pub struct ContentSourceNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentSourceNodeData<T>>,
}

pub struct ContentMiddleNodeData<T: RenderGraphBackend> {
  pub key: T::ContentMiddleKey,
}

pub struct ContentMiddleNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentMiddleNodeData<T>>,
}

impl<T: RenderGraphBackend> RenderGraph<T> {
  pub fn source(&self, key: T::ContentSourceKey) -> ContentSourceNodeBuilder<T> {
    let handle = self.create_node(RenderGraphNode::Source(ContentSourceNodeData { key }));

    ContentSourceNodeBuilder {
      builder: NodeBuilder::new(self, handle),
    }
  }
}
