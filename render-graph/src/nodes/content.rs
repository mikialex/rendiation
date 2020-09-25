use crate::{NodeBuilder, RenderGraphBackend, RenderGraphNode, RenderGraphNodeHandle};
use std::collections::HashSet;

pub enum ContentKey<T: RenderGraphBackend> {
  Source(T::ContentSourceKey),
  Inner(T::ContentMiddleKey),
}

pub struct ContentSourceNodeData<T: RenderGraphBackend> {
  pub key: T::ContentSourceKey,
}

pub struct ContentSourceNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentSourceNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> ContentSourceNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
}

pub struct ContentMiddleNodeData<T: RenderGraphBackend> {
  pub key: T::ContentMiddleKey,
  pub(crate) from: RenderGraphNodeHandle<T>,
}

pub struct ContentMiddleNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentMiddleNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> ContentMiddleNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
}

pub struct ContentTransformerNodeData<T: RenderGraphBackend> {
  read: HashSet<ContentKey<T>, RenderGraphNodeHandle<T>>,
  write: HashSet<T::ContentMiddleKey, RenderGraphNodeHandle<T>>,
  transformer: Box<
    dyn FnMut(
      &HashSet<ContentKey<T>, T::ContentUnitImpl>,
      &mut HashSet<ContentKey<T>, T::ContentUnitImpl>,
    ),
  >,
}

pub struct ContentTransformerNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentTransformerNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> ContentTransformerNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }

  pub fn transformer_data_mut(&self, mutator: impl FnOnce(&mut ContentTransformerNodeData<T>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data = graph.get_node_mut(self.handle()).data_mut();
    if let RenderGraphNode::Transformer(data) = data {
      mutator(data)
    }
  }

  pub fn read_source(self, source: &ContentSourceNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&source.builder);
    self.transformer_data_mut(|data| {
      // data.read.insert();
    });
    self
  }

  pub fn write(self, middle: &ContentMiddleNodeBuilder<'a, T>) -> Self {
    middle.builder.connect_from(&self.builder);
    self
  }
}
