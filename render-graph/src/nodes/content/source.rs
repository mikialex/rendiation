use crate::{
  ContentProvider, NodeBuilder, RenderGraphBackend, RenderGraphExecutor, RenderGraphNodeHandle,
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
