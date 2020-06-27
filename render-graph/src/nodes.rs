use crate::{RenderGraph, RenderGraphNodeHandle};
pub use rendiation_math::*;
pub use rendiation_render_entity::*;
use std::collections::HashSet;

pub enum RenderGraphNode {
  Pass(PassNodeData),
  Target(TargetNodeData),
}

pub struct PassNodeData {
  pub(crate) name: String,
  pub(crate) viewport: Viewport,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle>,
  pub(crate) render: Option<Box<dyn FnMut()>>,
}

pub struct TargetNodeData {
  pub name: String,
}

pub struct NodeBuilder<'a> {
  pub(crate) handle: RenderGraphNodeHandle,
  pub(crate) graph: &'a RenderGraph,
}

pub struct TargetNodeBuilder<'a> {
  pub(crate) builder: NodeBuilder<'a>,
}

impl<'a> TargetNodeBuilder<'a> {
  pub fn handle(&self) -> RenderGraphNodeHandle {
    self.builder.handle
  }
  pub fn from_pass(self, passes: &PassNodeBuilder<'a>) -> Self {
    self
  }
}

pub struct PassNodeBuilder<'a> {
  pub(crate) builder: NodeBuilder<'a>,
}

impl<'a> PassNodeBuilder<'a> {
  pub fn handle(&self) -> RenderGraphNodeHandle {
    self.builder.handle
  }

  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data_handle = graph.get_node(self.builder.handle).data_handle();
    let data = graph.get_node_data_mut(data_handle);
    if let RenderGraphNode::Pass(data) = data {
      mutator(data)
    }
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a>) -> Self {
    self.pass_data_mut(|p| {
      p.input_targets_map.insert(target.builder.handle);
    });
    self
  }

  pub fn viewport(self) -> Self {
    self.pass_data_mut(|p| {
      // p.viewport
    });
    self
  }
}
