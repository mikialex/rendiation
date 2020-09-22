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
}

// marco?
impl<T: RenderGraphBackend> RenderGraphNode<T> {
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
  pub fn unwrap_pass_data(&self) -> &PassNodeData<T> {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
  pub fn unwrap_pass_data_mut(&mut self) -> &mut PassNodeData<T> {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
}

pub struct NodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) handle: RenderGraphNodeHandle<T>,
  pub(crate) graph: &'a RenderGraph<T>,
}
