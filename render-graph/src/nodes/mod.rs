use crate::{ContentProvider, RenderGraph, RenderGraphBackend, RenderGraphNodeHandle};
pub use rendiation_math::*;
pub use rendiation_render_entity::*;

pub mod pass;
pub use pass::*;
pub mod target;
pub use target::*;
pub mod content;
pub use content::*;

pub enum RenderGraphNode<T: RenderGraphBackend, U: ContentProvider<T>> {
  Pass(PassNodeData<T, U>),
  Target(TargetNodeData<T>),
  Source(ContentSourceNodeData<T, U>),
}

// marco?
impl<T: RenderGraphBackend, U: ContentProvider<T>> RenderGraphNode<T, U> {
  pub fn is_pass(&self) -> bool {
    if let RenderGraphNode::Pass(_) = self {
      true
    } else {
      false
    }
  }
  pub fn unwrap_target_data(&self) -> &TargetNodeData<T> {
    if let RenderGraphNode::Target(data) = self {
      data
    } else {
      panic!("unwrap_as_target failed")
    }
  }
  pub fn unwrap_target_data_mut(&mut self) -> &mut TargetNodeData<T> {
    if let RenderGraphNode::Target(data) = self {
      data
    } else {
      panic!("unwrap_as_target failed")
    }
  }
  pub fn unwrap_pass_data(&self) -> &PassNodeData<T, U> {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
  pub fn unwrap_pass_data_mut(&mut self) -> &mut PassNodeData<T, U> {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
}

pub struct NodeBuilder<'a, T: RenderGraphBackend, U: ContentProvider<T>> {
  pub(crate) handle: RenderGraphNodeHandle<T, U>,
  pub(crate) graph: &'a RenderGraph<T, U>,
}
