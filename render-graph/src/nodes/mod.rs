use std::marker::PhantomData;

use crate::{RenderGraph, RenderGraphBackend, RenderGraphNodeHandle};
pub use rendiation_math::*;
pub use rendiation_render_entity::*;

pub mod pass;
pub use pass::*;
pub mod target;
pub use target::*;
pub mod content;
pub use content::*;

pub enum RenderGraphNode<T: RenderGraphBackend> {
  Pass(PassNodeData<T>),
  Target(TargetNodeData<T>),
  Source(ContentSourceNodeData<T>),
  Middle(ContentMiddleNodeData<T>),
  Transformer(ContentTransformerNodeData<T>),
}

impl<T: RenderGraphBackend> RenderGraphNode<T> {
  pub fn downcast<U: FromRenderGraphNode<T>>(&self) -> Option<&U> {
    FromRenderGraphNode::downcast(self)
  }
  pub fn downcast_mut<U: FromRenderGraphNode<T>>(&mut self) -> Option<&mut U> {
    FromRenderGraphNode::downcast_mut(self)
  }
  pub fn is_pass(&self) -> bool {
    if let RenderGraphNode::Pass(_) = self {
      true
    } else {
      false
    }
  }
}

pub trait FromRenderGraphNode<T: RenderGraphBackend> {
  fn downcast_mut(node: &mut RenderGraphNode<T>) -> Option<&mut Self>;
  fn downcast(node: &RenderGraphNode<T>) -> Option<&Self>;
}

macro_rules! impl_downcast {
  ($NodeData:ident, $Enum:ident) => {
    impl<T: RenderGraphBackend> FromRenderGraphNode<T> for $NodeData<T> {
      fn downcast_mut(node: &mut RenderGraphNode<T>) -> Option<&mut Self> {
        if let RenderGraphNode::$Enum(data) = node {
          Some(data)
        } else {
          None
        }
      }
      fn downcast(node: &RenderGraphNode<T>) -> Option<&Self> {
        if let RenderGraphNode::$Enum(data) = node {
          Some(data)
        } else {
          None
        }
      }
    }
  };
}

impl_downcast!(TargetNodeData, Target);
impl_downcast!(PassNodeData, Pass);
impl_downcast!(ContentSourceNodeData, Source);
impl_downcast!(ContentMiddleNodeData, Middle);
impl_downcast!(ContentTransformerNodeData, Transformer);

pub struct NodeBuilder<'a, T: RenderGraphBackend, U: FromRenderGraphNode<T>> {
  pub(crate) handle: RenderGraphNodeHandle<T>,
  pub(crate) graph: &'a RenderGraph<T>,
  pub(crate) phantom: PhantomData<U>,
}

impl<'a, T: RenderGraphBackend, U: FromRenderGraphNode<T>> NodeBuilder<'a, T, U> {
  pub fn mutate_data(&self, mutator: impl FnOnce(&mut U)) -> &Self {
    let mut graph = self.graph.graph.borrow_mut();
    let data = graph.get_node_mut(self.handle).data_mut();
    U::downcast_mut(data).map(mutator);
    self
  }

  pub fn connect_from<X: FromRenderGraphNode<T>>(&self, other: &NodeBuilder<'a, T, X>) -> &Self {
    self
      .graph
      .graph
      .borrow_mut()
      .connect_node(other.handle, self.handle);
    self
  }
}
