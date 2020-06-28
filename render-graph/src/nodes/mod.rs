use crate::{RenderGraph, RenderGraphNodeHandle};
pub use rendiation_math::*;
pub use rendiation_render_entity::*;

pub mod pass;
pub use pass::*;
pub mod target;
pub use target::*;

pub enum RenderGraphNode {
  Pass(PassNodeData),
  Target(TargetNodeData),
}

// marco?
impl RenderGraphNode {
  pub fn is_pass(&self) -> bool {
    if let RenderGraphNode::Pass(_) = self {
      true
    } else {
      false
    }
  }
  pub fn unwrap_target_data(&self) -> &TargetNodeData {
    if let RenderGraphNode::Target(data) = self {
      data
    } else {
      panic!("unwrap_as_target failed")
    }
  }
  pub fn unwrap_pass_data(&self) -> &PassNodeData {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
  pub fn unwrap_pass_data_mut(&mut self) -> &mut PassNodeData {
    if let RenderGraphNode::Pass(data) = self {
      data
    } else {
      panic!("unwrap_pass_data failed")
    }
  }
}

pub struct NodeBuilder<'a> {
  pub(crate) handle: RenderGraphNodeHandle,
  pub(crate) graph: &'a RenderGraph,
}
