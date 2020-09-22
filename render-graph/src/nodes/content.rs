use crate::{NodeBuilder, RenderGraphBackend, RenderGraphNodeHandle, RootContentProvider};

pub struct ContentSourceNodeData<T: RenderGraphBackend> {
  name: String,
  source: Box<dyn FnMut(&mut Box<dyn RootContentProvider<T>>) -> T::PassContentProvider>,
}

pub struct ContentNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> ContentNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
}
