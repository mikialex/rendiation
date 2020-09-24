use crate::{ContentProvider, NodeBuilder, RenderGraphBackend, RenderGraphNodeHandle};

pub struct ContentSourceNodeData<T: RenderGraphBackend> {
  name: T::ContentKey,
}

pub struct ContentNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> ContentNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
}
