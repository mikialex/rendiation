use crate::{NodeBuilder, RenderGraphBackend, RenderGraphNodeHandle};
use std::{collections::HashMap, hash::Hash};

pub enum ContentKey<T: RenderGraphBackend> {
  Source(T::ContentSourceKey),
  Inner(T::ContentMiddleKey),
}

impl<T: RenderGraphBackend> PartialEq for ContentKey<T> {
  fn eq(&self, other: &Self) -> bool {
    self == other
  }
}
impl<T: RenderGraphBackend> Eq for ContentKey<T> {}
impl<T: RenderGraphBackend> Copy for ContentKey<T> {}
impl<T: RenderGraphBackend> Clone for ContentKey<T> {
  fn clone(&self) -> Self {
    match self {
      ContentKey::Source(s) => ContentKey::Source(*s),
      ContentKey::Inner(i) => ContentKey::Inner(*i),
    }
  }
}
impl<T: RenderGraphBackend> Hash for ContentKey<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      ContentKey::Source(s) => s.hash(state),
      ContentKey::Inner(i) => i.hash(state),
    }
  }
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
  pub read: HashMap<ContentKey<T>, RenderGraphNodeHandle<T>>,
  pub write: HashMap<T::ContentMiddleKey, RenderGraphNodeHandle<T>>,
  pub transformer: Box<
    dyn FnMut(
      &HashMap<ContentKey<T>, T::ContentUnitImpl>,
      &mut HashMap<T::ContentMiddleKey, T::ContentUnitImpl>,
    ),
  >,
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
