use crate::{NodeBuilder, RenderGraphBackend, RenderGraphNode, RenderGraphNodeHandle};
use std::collections::HashMap;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
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
  read: HashMap<ContentKey<T>, RenderGraphNodeHandle<T>>,
  write: HashMap<T::ContentMiddleKey, RenderGraphNodeHandle<T>>,
  transformer: Box<
    dyn FnMut(
      &HashMap<ContentKey<T>, T::ContentUnitImpl>,
      &mut HashMap<ContentKey<T>, T::ContentUnitImpl>,
    ),
  >,
}

pub struct ContentTransformerNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentTransformerNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> ContentTransformerNodeBuilder<'a, T> {
  pub fn read_source(self, source: &ContentSourceNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&source.builder);
    let handle = source.builder.handle;
    let key;
    source
      .builder
      .mutate_data(|d| key = ContentKey::Source(d.key));
    self.builder.mutate_data(|data| {
      data.read.insert(key, source.builder.handle);
    });
    self
  }

  pub fn read_middle(self, source: &ContentMiddleNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&source.builder);
    let handle = source.builder.handle;
    let key;
    source
      .builder
      .mutate_data(|d| key = ContentKey::Inner(d.key));
    self.builder.mutate_data(|data| {
      data.read.insert(key, source.builder.handle);
    });
    self
  }

  pub fn write(self, middle: &ContentMiddleNodeBuilder<'a, T>) -> Self {
    middle.builder.connect_from(&self.builder);
    let key;
    middle.builder.mutate_data(|d| key = d.key);
    self.builder.mutate_data(|data| {
      data.write.insert(key, middle.builder.handle);
    });
    self
  }
}
