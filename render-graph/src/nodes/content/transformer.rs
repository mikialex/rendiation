use std::collections::HashMap;

use crate::{
  ContentKey, ContentMiddleNodeBuilder, ContentSourceNodeBuilder, NodeBuilder, RenderGraphBackend,
  RenderGraphExecutor, RenderGraphNodeHandle,
};

pub struct ContentTransformerNodeData<T: RenderGraphBackend> {
  pub read: HashMap<ContentKey<T>, RenderGraphNodeHandle<T>>,
  pub write: HashMap<T::ContentMiddleKey, RenderGraphNodeHandle<T>>,
  pub transformer: Box<
    dyn Fn(
      &HashMap<ContentKey<T>, T::ContentUnitImpl>,
      &mut HashMap<T::ContentMiddleKey, T::ContentUnitImpl>,
    ),
  >,
}

impl<T: RenderGraphBackend> ContentTransformerNodeData<T> {
  pub fn execute(&self, executor: &mut RenderGraphExecutor<T>) {
    let mut read_map = self
      .read
      .iter()
      .map(|(&key, &value)| (key, executor.content_pool.load(value)))
      .collect();

    let mut write_map = self
      .write
      .iter()
      .map(|(&key, &value)| (key, executor.content_pool.load(value)))
      .collect();

    (self.transformer)(&read_map, &mut write_map);

    read_map.drain().for_each(|(key, content)| {
      executor
        .content_pool
        .store(*self.read.get(&key).unwrap(), content);
    });

    write_map.drain().for_each(|(key, content)| {
      executor
        .content_pool
        .store(*self.write.get(&key).unwrap(), content);
    });
  }
}

pub struct ContentTransformerNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, ContentTransformerNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> ContentTransformerNodeBuilder<'a, T> {
  pub fn read_source(self, source: &ContentSourceNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&source.builder);
    let mut key = None;
    source
      .builder
      .mutate_data(|d| key = Some(ContentKey::Source(d.key)));
    self.builder.mutate_data(|data| {
      data.read.insert(key.unwrap(), source.builder.handle);
    });
    self
  }

  pub fn read_middle(self, source: &ContentMiddleNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&source.builder);
    let mut key = None;
    source
      .builder
      .mutate_data(|d| key = Some(ContentKey::Inner(d.key)));
    self.builder.mutate_data(|data| {
      data.read.insert(key.unwrap(), source.builder.handle);
    });
    self
  }

  pub fn write(self, middle: &ContentMiddleNodeBuilder<'a, T>) -> Self {
    middle.builder.connect_from(&self.builder);
    let mut key = None;
    middle.builder.mutate_data(|d| key = Some(d.key));
    self.builder.mutate_data(|data| {
      data.write.insert(key.unwrap(), middle.builder.handle);
    });
    self
  }
}
