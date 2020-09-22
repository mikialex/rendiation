use crate::{
  ContentProvider, NodeBuilder, RenderGraphBackend, RenderGraphNodeHandle, RootContentProvider,
};

pub struct ContentSourceNodeData<T: RenderGraphBackend, U: ContentProvider<T>> {
  name: String,
  source: Box<dyn FnMut(&mut Box<dyn RootContentProvider<T, U>>) -> U>,
}

pub struct ContentNodeBuilder<'a, T: RenderGraphBackend, U: ContentProvider<T>> {
  pub(crate) builder: NodeBuilder<'a, T, U>,
}

impl<'a, T: RenderGraphBackend, U: ContentProvider<T>> ContentNodeBuilder<'a, T, U> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T, U> {
    self.builder.handle
  }
}
